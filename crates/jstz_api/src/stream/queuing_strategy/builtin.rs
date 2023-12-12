use boa_engine::{
    js_string, object::Object, property::Attribute, value::TryFromJs, Context, JsArgs,
    JsNativeError, JsResult, JsValue, NativeFunction,
};
use boa_gc::{Finalize, GcRefMut, Trace};
use jstz_core::{
    accessor,
    native::{Accessor, ClassBuilder, JsNativeObject, NativeClass},
};

use crate::{
    idl,
    stream::{tmp::get_JsObject_property, Chunk},
};

use super::{size::SizeAlgorithm, CountQueuingStrategy, HighWaterMarkAndSizeAlgorithm};

/// [Streams Standard - § 7.1.][https://streams.spec.whatwg.org/#qs-api]
/// > ```
/// > dictionary QueuingStrategyInit {
/// >   required unrestricted double highWaterMark;
/// > };
/// > ```
pub struct QueuingStrategyInit {
    pub high_water_mark: idl::UnrestrictedDouble,
}

impl TryFromJs for QueuingStrategyInit {
    fn try_from_js(value: &JsValue, context: &mut Context<'_>) -> JsResult<Self> {
        let this = value.to_object(context)?;
        let high_water_mark: idl::UnrestrictedDouble =
            get_JsObject_property(&this, "highWaterMark", context)?
                .try_js_into::<Option<idl::UnrestrictedDouble>>(context)?
                .expect("Missing highWaterMark property");
        // TODO: .try_js_into::<Option<idl::UnrestrictedDouble>>(context) is wrong. "qwe" should map to "NaN"

        Ok(QueuingStrategyInit { high_water_mark })
    }
}

impl<U: Default> HighWaterMarkAndSizeAlgorithm<idl::UnrestrictedDouble, U> {
    pub fn new(init: QueuingStrategyInit) -> Self {
        HighWaterMarkAndSizeAlgorithm {
            high_water_mark: init.high_water_mark,
            size_algorithm: U::default(),
        }
    }
}

impl CountQueuingStrategy {
    pub fn try_from_js<'a>(value: &'a JsValue) -> JsResult<GcRefMut<'a, Object, Self>> {
        value
            .as_object()
            .and_then(|obj| obj.downcast_mut::<Self>())
            .ok_or_else(|| {
                JsNativeError::typ()
                    .with_message(
                        "Failed to convert js value into Rust type `QueuingStrategy`",
                    )
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
            get:((strategy, _context) => Ok(strategy.high_water_mark.into()))
        )
    }

    fn size(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context<'_>,
    ) -> JsResult<JsValue> {
        let strategy = CountQueuingStrategy::try_from_js(this)?;
        let chunk = args.get_or_undefined(0);
        strategy.size_algorithm.call(chunk, context).map(Into::into)
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

// TODO: implement same things for ByteLength (without duplicating code if possible)
