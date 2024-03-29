use std::{
    collections::{btree_map, BTreeMap, BTreeSet},
    marker::PhantomData,
    mem,
};

use derive_more::{Deref, DerefMut};
use serde::de::DeserializeOwned;
use tezos_smart_rollup_host::{path::OwnedPath, runtime::Runtime};

use super::value::{BoxedValue, Value};
use super::Storage;
use crate::error::{KvError, Result};

/// A transaction is a 'lazy' snapshot of the persistent key-value store from
/// the point in time when the transaction began. Modifications to new or old
/// values within the transaction remain isolated from any concurrent
/// transactions.
///
/// Reads are cached for each transaction, optimizing repeated accesses to the
/// same key. Writes are buffered in using an in-memory representation until the
/// root transaction is successfully committed, at which point the buffer is flushed
/// to the persistent storage.
///
/// Transactions offer ACID guarentees. The weakest property for these gaurentees
/// to hold is [serializability](https://en.wikipedia.org/wiki/Serializability), ensuring
/// that a transaction can only be committed if it does not conflict with a
/// previously committed transaction. For example, if a transaction `t1` reads any key-value
/// pair that is modified and committed in a later transaction `t2` before `t1` is comitted,
/// `t1` will fail. In other words, the following transaction behaviour will lead to a
/// conflict:
///
/// ```text
/// +- t1: ---------+
/// | read key1     |   +- t2 ----------+
/// |               |   | write key1    |
/// |               |   | commit: true  |
/// | write key1    |   +---------------+
/// | commit: false |
/// +---------------+
/// ```
///
/// NOTE: Current implementation does NOT support concurrent transactions

/// A key is a path in durable storage
pub type Key = OwnedPath;

// A lookup map is a history of edits of a given key in order of least-recent to most-recent
// This allows O(log n) lookups, and O(log n) commits / rollbacks (amortized by # of inserts / removals).
#[derive(Debug, Default, Deref, DerefMut)]
struct LookupMap(BTreeMap<Key, Vec<usize>>);

#[derive(Debug, Default)]
pub struct Transaction {
    // A stack of transactional snapshots
    stack: Vec<Snapshot>,
    lookup_map: LookupMap,
}

#[derive(Debug, Clone, Deref, DerefMut)]
struct SnapshotValue(BoxedValue);

impl SnapshotValue {
    pub fn new(value: impl Value) -> Self {
        Self(BoxedValue::new(value))
    }

    pub fn as_ref<V>(&self) -> Result<&V>
    where
        V: Value,
    {
        Ok(self
            .as_any()
            .downcast_ref()
            .ok_or(KvError::DowncastFailed)?)
    }

    pub fn as_mut<V>(&mut self) -> Result<&mut V>
    where
        V: Value,
    {
        Ok(self
            .as_any_mut()
            .downcast_mut()
            .ok_or(KvError::DowncastFailed)?)
    }

    pub fn into_value<V>(self) -> Result<V>
    where
        V: Value,
    {
        let value = self.0.downcast().map_err(|_| KvError::DowncastFailed)?;
        *value
    }
}

#[derive(Debug, Default)]
struct Snapshot {
    // INVARIANT: Set of keys in the edits are disjoint
    // A map of 'insert' edits to be applied
    insert_edits: BTreeMap<Key, SnapshotValue>,
    // A set of 'remove' edits to be applied
    remove_edits: BTreeSet<Key>,
}

impl Snapshot {
    pub fn insert(&mut self, key: Key, value: SnapshotValue) {
        self.remove_edits.remove(&key);
        self.insert_edits.insert(key, value);
    }

    pub fn remove(&mut self, key: Key) {
        self.insert_edits.remove(&key);
        self.remove_edits.insert(key);
    }

    pub fn lookup(&self, key: &Key) -> Option<&SnapshotValue> {
        if self.remove_edits.contains(key) {
            return None;
        }

        self.insert_edits.get(key)
    }

    pub fn lookup_mut(&mut self, key: &Key) -> Option<&mut SnapshotValue> {
        if self.remove_edits.contains(key) {
            return None;
        }

        self.insert_edits.get_mut(key)
    }

    pub fn contains_key(&self, key: &Key) -> bool {
        self.insert_edits.contains_key(key) && !self.remove_edits.contains(key)
    }
}

impl LookupMap {
    fn update(&mut self, key: Key, idx: usize) {
        let key_history = self.entry(key).or_default();

        match key_history.last() {
            Some(&last_idx) if last_idx == idx => {
                // The key was already looked up in the current context
            }
            _ => {
                key_history.push(idx);
            }
        }
    }

    fn rollback(&mut self, key: &Key) -> Result<()> {
        let is_history_empty = {
            let history = self.get_mut(key).ok_or(KvError::ExpectedLookupMapEntry)?;

            history.pop();
            history.is_empty()
        };

        if is_history_empty {
            self.remove(key);
        }

        Ok(())
    }
}

impl Transaction {
    fn current_snapshot_idx(&self) -> usize {
        self.stack.len().saturating_sub(1)
    }

    fn update_lookup_map(&mut self, key: Key) {
        self.lookup_map.update(key, self.current_snapshot_idx())
    }

    /// Return the current snapshot
    fn current_snapshot(&mut self) -> Result<&mut Snapshot> {
        Ok(self
            .stack
            .last_mut()
            .ok_or(KvError::TransactionStackEmpty)?)
    }

    /// Insert a key-value pair into the current snapshot (as a 'insert' edit)
    fn current_snapshot_insert(&mut self, key: Key, value: SnapshotValue) -> Result<()> {
        self.update_lookup_map(key.clone());
        self.current_snapshot()?.insert(key, value);
        Ok(())
    }

    /// Lookup a key in the current snapshot
    fn current_snapshot_lookup(&mut self, key: &Key) -> Result<Option<&SnapshotValue>> {
        Ok(self.current_snapshot()?.lookup(key))
    }

    /// Lookup a key in the current snapshot
    fn current_snapshot_lookup_mut(
        &mut self,
        key: &Key,
    ) -> Result<Option<&mut SnapshotValue>> {
        Ok(self.current_snapshot()?.lookup_mut(key))
    }

    /// Remove a key from the current snapshot (as a 'remove' edit)
    fn current_snapshot_remove(&mut self, key: Key) -> Result<()> {
        self.update_lookup_map(key.clone());
        self.current_snapshot()?.remove(key);
        Ok(())
    }

    fn lookup<V>(&mut self, rt: &impl Runtime, key: Key) -> Result<Option<&SnapshotValue>>
    where
        V: Value + DeserializeOwned,
    {
        if let Some(&snapshot_idx) =
            self.lookup_map.get(&key).and_then(|history| history.last())
        {
            let snapshot = &self.stack[snapshot_idx];

            return Ok(snapshot.lookup(&key));
        }

        if let Some(value) = Storage::get::<V>(rt, &key)? {
            // TODO: This clone is probably not necessary
            self.current_snapshot_insert(key.clone(), SnapshotValue::new(value))?;

            self.current_snapshot_lookup(&key)
        } else {
            Ok(None)
        }
    }

    fn lookup_mut<V>(
        &mut self,
        rt: &impl Runtime,
        key: Key,
    ) -> Result<Option<&mut SnapshotValue>>
    where
        V: Value + DeserializeOwned,
    {
        if let Some(&snapshot_idx) =
            self.lookup_map.get(&key).and_then(|history| history.last())
        {
            let snapshot = &self.stack[snapshot_idx];

            if let Some(value) = snapshot.lookup(&key) {
                self.current_snapshot_insert(key.clone(), value.clone())?;
                self.current_snapshot_lookup_mut(&key)
            } else {
                Ok(None)
            }
        } else if let Some(value) = Storage::get::<V>(rt, &key)? {
            self.current_snapshot_insert(key.clone(), SnapshotValue::new(value))?;
            self.current_snapshot_lookup_mut(&key)
        } else {
            Ok(None)
        }
    }

    /// Returns a reference to the value corresponding to the key in the
    /// key-value store if it exists.
    pub fn get<V>(&mut self, rt: &impl Runtime, key: Key) -> Result<Option<&V>>
    where
        V: Value + DeserializeOwned,
    {
        self.lookup::<V>(rt, key)
            .map(|entry_opt| entry_opt.map(|entry| entry.as_ref()).transpose())?
    }

    /// Returns a mutable reference to the value corresponding to the key in the
    /// key-value store if it exists.
    pub fn get_mut<V>(&mut self, rt: &impl Runtime, key: Key) -> Result<Option<&mut V>>
    where
        V: Value + DeserializeOwned,
    {
        self.lookup_mut::<V>(rt, key)
            .map(|entry_opt| entry_opt.map(|entry| entry.as_mut()).transpose())?
    }

    /// Returns `true` if the key-value store contains a key-value pair for the
    /// specified key.
    pub fn contains_key(&self, rt: &impl Runtime, key: &Key) -> Result<bool> {
        if let Some(&context_idx) =
            self.lookup_map.get(key).and_then(|history| history.last())
        {
            let context = &self.stack[context_idx];

            return Ok(context.contains_key(key));
        }

        Storage::contains_key(rt, key)
    }

    /// Insert a key-value pair into the key-value store.
    pub fn insert<V>(&mut self, key: Key, value: V) -> Result<()>
    where
        V: Value,
    {
        self.current_snapshot_insert(key, SnapshotValue::new(value))
    }

    /// Removes a key from the key-value store.
    pub fn remove(&mut self, key: Key) -> Result<()> {
        self.current_snapshot_remove(key)
    }

    /// Returns the given key's corresponding entry in the transactional
    /// snapshot for in-place manipulation.
    pub fn entry<'a, 'b, V>(
        &'a mut self,
        rt: &impl Runtime,
        key: Key,
    ) -> Result<Entry<'b, V>>
    where
        V: Value + DeserializeOwned,
        'a: 'b,
    {
        // A mutable lookup ensures the key is in the current snapshot
        self.lookup_mut::<V>(rt, key.clone())?;

        let current_snapshot_idx = self.current_snapshot_idx();
        // self.current_snapshot() inlined to avoid lifetime issue
        let current_snapshot = self
            .stack
            .last_mut()
            .ok_or(KvError::TransactionStackEmpty)?;

        match current_snapshot.insert_edits.entry(key) {
            btree_map::Entry::Vacant(inner) => Ok(Entry::vacant(
                inner,
                &mut self.lookup_map,
                current_snapshot_idx,
            )),
            btree_map::Entry::Occupied(inner) => {
                Ok(Entry::occupied(inner, &mut current_snapshot.remove_edits))
            }
        }
    }

    /// Begin a transaction.
    pub fn begin(&mut self) {
        self.stack.push(Snapshot::default())
    }

    /// Commit a transaction.
    pub fn commit(&mut self, rt: &mut impl Runtime) -> Result<()> {
        let curr_ctxt = self.stack.pop().ok_or(KvError::TransactionStackEmpty)?;

        // Following the `.pop`, `prev_idx` is the index of prev_idx (if it exists)
        let prev_idx = self.current_snapshot_idx();

        if let Some(prev_ctxt) = self.stack.last_mut() {
            // TODO: These clones are probably uncessary since the entry of btree will always be occupied.
            for key in curr_ctxt.remove_edits {
                self.lookup_map.update(key.clone(), prev_idx);
                prev_ctxt.remove(key);
            }

            for (key, value) in curr_ctxt.insert_edits {
                self.lookup_map.update(key.clone(), prev_idx);
                prev_ctxt.insert(key, value);
            }
        } else {
            for key in &curr_ctxt.remove_edits {
                Storage::remove(rt, key)?
            }

            for (key, value) in curr_ctxt.insert_edits {
                Storage::insert(rt, &key, value.0.as_ref())?
            }

            // Update lookup map
            self.lookup_map.clear()
        }

        Ok(())
    }

    /// Rollback a transaction.
    pub fn rollback(&mut self) -> Result<()> {
        let curr_ctxt = self.stack.pop().ok_or(KvError::TransactionStackEmpty)?;

        // SAFETY: The set of keys between removal edits and insertion edits are disjoint, meaning no
        // `lookup_map` entries will be rolledback more than once
        for key in &curr_ctxt.remove_edits {
            self.lookup_map.rollback(key)?;
        }

        for key in curr_ctxt.insert_edits.keys() {
            self.lookup_map.rollback(key)?
        }

        Ok(())
    }
}

/// A view into a single entry in the transaction snapshot, which is either
/// vacant or occupied.
pub enum Entry<'a, V: 'a> {
    /// A vacant entry.
    Vacant(VacantEntry<'a, V>),

    /// An occupied entry.
    Occupied(OccupiedEntry<'a, V>),
}

impl<'a, V> Entry<'a, V> {
    fn vacant(
        inner: btree_map::VacantEntry<'a, Key, SnapshotValue>,
        lookup_map: &'a mut LookupMap,
        snapshot_idx: usize,
    ) -> Self {
        Entry::Vacant(VacantEntry {
            inner,
            lookup_map,
            snapshot_idx,
            _marker: PhantomData,
        })
    }

    fn occupied(
        inner: btree_map::OccupiedEntry<'a, Key, SnapshotValue>,
        remove_edits: &'a mut BTreeSet<Key>,
    ) -> Self {
        Entry::Occupied(OccupiedEntry {
            inner,
            remove_edits,
            _marker: PhantomData,
        })
    }

    pub fn or_insert_default(self) -> &'a mut V
    where
        V: Value + Default,
    {
        match self {
            Entry::Vacant(vacant_entry) => vacant_entry.insert(Default::default()),
            Entry::Occupied(occupied_entry) => occupied_entry.into_mut(),
        }
    }
}

/// A view into a vacant entry in the transactional snapshot.
pub struct VacantEntry<'a, V: 'a> {
    inner: btree_map::VacantEntry<'a, Key, SnapshotValue>,
    // Reference to lookup map (if we insert into the vacant entry)
    lookup_map: &'a mut LookupMap,
    snapshot_idx: usize,
    _marker: PhantomData<V>,
}

impl<'a, V: 'a> VacantEntry<'a, V> {
    /// Gets a reference to the key of the entry.
    pub fn key(&self) -> &Key {
        self.inner.key()
    }

    /// Take ownership of the key.
    pub fn into_key(self) -> Key {
        self.inner.into_key()
    }

    /// Set the value of the entry using the entry's key and return a mutable
    /// reference to the value.
    pub fn insert(self, value: V) -> &'a mut V
    where
        V: Value,
    {
        self.lookup_map
            .update(self.key().clone(), self.snapshot_idx);
        self.inner
            .insert(SnapshotValue::new(value))
            .as_mut()
            .expect("Invalid type id invariant")
    }
}

/// A view into an occupied entry in the transactional snapshot.

pub struct OccupiedEntry<'a, V: 'a> {
    inner: btree_map::OccupiedEntry<'a, Key, SnapshotValue>,
    // Reference to the set of keys to be removed from the current snapshot
    remove_edits: &'a mut BTreeSet<Key>,
    _marker: PhantomData<V>,
}

impl<'a, V> OccupiedEntry<'a, V> {
    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> &Key {
        self.inner.key()
    }

    /// Takes the key-value pair out of the snapshot, returning ownership
    /// to the caller.
    pub fn remove_entry(self) -> (Key, V)
    where
        V: Value,
    {
        let (key, entry) = self.inner.remove_entry();
        self.remove_edits.insert(key.clone());
        (key, entry.into_value().expect("Invalid type id invariant"))
    }

    /// Gets a reference to the value in the entry.
    pub fn get(&self) -> &V
    where
        V: Value,
    {
        self.inner
            .get()
            .as_ref()
            .expect("Invalid type id invariant")
    }

    /// Get a mutable reference to the value in the entry.
    pub fn get_mut(&mut self) -> &mut V
    where
        V: Value,
    {
        self.inner
            .get_mut()
            .as_mut()
            .expect("Invalid type id invariant")
    }

    /// Convert the entry into a mutable reference to its value.
    pub fn into_mut(self) -> &'a mut V
    where
        V: Value,
    {
        self.inner
            .into_mut()
            .as_mut()
            .expect("Invalid type id invariant")
    }

    /// Sets the value of the entry and returns the entry's old value.
    pub fn insert(&mut self, value: V) -> V
    where
        V: Value,
    {
        std::mem::replace(self.get_mut(), value)
    }

    /// Take the value of the entry out of the snapshot, and return it.
    pub fn remove(self) -> V
    where
        V: Value,
    {
        self.remove_edits.insert(self.key().clone());
        self.inner
            .remove()
            .into_value()
            .expect("Invalid type id invariant")
    }
}

#[derive(Debug, Deref, DerefMut)]
pub struct JsTransaction {
    inner: &'static mut Transaction,
}

impl JsTransaction {
    pub unsafe fn new(tx: &mut Transaction) -> Self {
        // SAFETY
        // From the pov of the `JsTransaction` struct, it is permitted to cast
        // the `tx` reference to `'static` since the lifetime of `JsTransaction`
        // is always shorter than the lifetime of `tx`

        let rt: &'static mut Transaction = mem::transmute(tx);

        Self { inner: rt }
    }
}
