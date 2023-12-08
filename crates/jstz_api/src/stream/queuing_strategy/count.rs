//! [Streams Standard - § 7.3. The CountQueuingStrategy class][https://streams.spec.whatwg.org/#cqs-class]

use boa_engine::{
    js_string, object::Object, property::Attribute, value::TryFromJs, Context, JsArgs,
    JsNativeError, JsResult, JsValue, NativeFunction,
};
use boa_gc::{empty_trace, Finalize, GcRefMut, Trace};
use jstz_core::{
    accessor,
    native::{Accessor, ClassBuilder, JsNativeObject, NativeClass},
};

use crate::{
    idl,
    stream::{readable::underlying_source::UnderlyingSource, Chunk},
};

use super::{other::OtherQueuingStrategy, QueuingStrategyInit};

/// Streams Standard - § 7.3. The CountQueuingStrategy class][https://streams.spec.whatwg.org/#cqs-class]
/// > A common queuing strategy when dealing with streams of generic objects is to simply count the number of chunks that have been accumulated so far, waiting until this number reaches a specified high-water mark. As such, this strategy is also provided out of the box.
///
/// [Streams Standard - § 7.3.1.][https://streams.spec.whatwg.org/#countqueuingstrategy]
/// > ```
/// > [Exposed=*]
/// > interface CountQueuingStrategy {
/// >   constructor(QueuingStrategyInit init);
/// >   
/// >   readonly attribute unrestricted double highWaterMark;
/// >   readonly attribute Function size;
/// > };
/// > ```
///
///

pub struct CountQueuingStrategy {
    high_water_mark: idl::UnrestrictedDouble,
}

impl Finalize for CountQueuingStrategy {
    fn finalize(&self) {}
}

unsafe impl Trace for CountQueuingStrategy {
    empty_trace!();
}

impl CountQueuingStrategy {
    pub fn new(init: QueuingStrategyInit) -> Self {
        CountQueuingStrategy {
            high_water_mark: init.high_water_mark,
        }
    }

    pub fn high_water_mark(&self) -> idl::UnrestrictedDouble {
        self.high_water_mark
    }

    const SIZE: idl::UnrestrictedDouble = 1.0;

    pub fn size(&self, _chunk: &Chunk) -> idl::UnrestrictedDouble {
        Self::SIZE
    }
}

impl CountQueuingStrategy {
    pub fn try_from_js<'a>(value: &'a JsValue) -> JsResult<GcRefMut<'a, Object, Self>> {
        value
            .as_object()
            .and_then(|obj| obj.downcast_mut::<Self>())
            .ok_or_else(|| {
                JsNativeError::typ()
                    .with_message("Failed to convert js value into rust type `CountQueuingStrategy`")
                    .into()
            })
    }
}

pub struct CountQueuingStrategyClass {}

impl CountQueuingStrategyClass {
    fn high_water_mark(context: &mut Context<'_>) -> Accessor {
        accessor!(
            context,
            CountQueuingStrategy,
            "highWaterMark",
            get:((strategy, _context) => Ok(strategy.high_water_mark().into()))
        )
    }

    fn size(
        _this: &JsValue,
        _args: &[JsValue],
        _context: &mut Context<'_>,
    ) -> JsResult<JsValue> {
        Ok(CountQueuingStrategy::SIZE.into())
    }
}

impl NativeClass for CountQueuingStrategyClass {
    type Instance = CountQueuingStrategy;

    const NAME: &'static str = "CountQueuingStrategy";

    fn constructor(
        _this: &JsNativeObject<Self::Instance>,
        args: &[boa_engine::JsValue],
        context: &mut Context<'_>,
    ) -> JsResult<Self::Instance> {
        let init: QueuingStrategyInit = args
            .get(0)
            .expect("The constructor of CountQueuingStrategy requires one argument")
            .try_js_into(context)?;
        Ok(CountQueuingStrategy::new(init))
    }

    fn init(class: &mut ClassBuilder<'_, '_>) -> JsResult<()> {
        let high_water_mark = Self::high_water_mark(class.context());

        class
            .accessor(
                js_string!("highWaterMark"),
                high_water_mark,
                Attribute::READONLY | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            )
            .method(
                js_string!("size"),
                1,
                NativeFunction::from_fn_ptr(Self::size),
            );
        Ok(())
    }
}
