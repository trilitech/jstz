use jstz_crypto::hash::Blake2b;
use tezos_crypto_rs::hash::ContractKt1Hash;
use tezos_smart_rollup::{
    michelson::{
        ticket::{FA2_1Ticket, Ticket},
        MichelsonBytes, MichelsonNat, MichelsonOption, MichelsonPair,
    },
    types::Contract,
};

pub mod host;
pub mod message;

pub fn parse_ticket(
    ticketer: ContractKt1Hash,
    amount: u32,
    content: (u32, Option<Vec<u8>>),
) -> FA2_1Ticket {
    let ticket_content = MichelsonPair(
        MichelsonNat::from(content.0),
        MichelsonOption::<MichelsonBytes>(content.1.clone().map(MichelsonBytes)),
    );

    Ticket::new(
        Contract::Originated(ticketer.clone()),
        ticket_content,
        amount,
    )
    .unwrap()
}

pub fn account1() -> jstz_crypto::public_key_hash::PublicKeyHash {
    jstz_crypto::public_key_hash::PublicKeyHash::from_base58(
        "tz1KqTpEZ7Yob7QbPE4Hy4Wo8fHG8LhKxZSx",
    )
    .unwrap()
}

pub fn account2() -> jstz_crypto::public_key_hash::PublicKeyHash {
    jstz_crypto::public_key_hash::PublicKeyHash::from_base58(
        "tz1QcqnzZ8pa6VuE4MSeMjsJkiW94wNrPbgX",
    )
    .unwrap()
}

pub fn ticket_hash1() -> Blake2b {
    let data = vec![b'0', b'0', b'0'];
    Blake2b::from(&data)
}
