//! [Streams Standard - § 7. Queuing strategies][https://streams.spec.whatwg.org/#qs]

use boa_engine::{value::TryFromJs, Context, JsNativeError, JsObject, JsResult, JsValue};
use jstz_core::{js_fn::JsFn, native::register_global_class, value::JsUndefined};

use crate::{idl, stream::Chunk, todo::Todo};

use self::{
    count::{CountQueuingStrategy, CountQueuingStrategyClass},
    other::OtherQueuingStrategy,
};

use super::tmp::get_JsObject_property;

pub mod count;
pub mod other;

/*
Important for design choice:
- strategies that are not instances of CoutQueuingStrategy or ByteLengthQueuingStrategy may not have a highWaterMark, but this property is never used directly. Instead, the function ExtractHighWaterMark(strategy, defaultHWM) is used, with different defaults depending on the call site. Using ExtractHighWaterMark is not very idiomatic in Rust, but we don't know which default to use when creating the object
- size is provided by a getter attached to the prototype for instances of CoutQueuingStrategy or ByteLengthQueuingStrategy, but can be an arbitrary function for other strategies. We therefore a priori want to keep distinguishing them by having a enum with 3 variants, with a trait implemented by each component, and on the enum by dispatching.

The choice of the Trait is not immediate:
- we'd want to take into account the potential lack of highWaterMark, and hence have the trait basically implement the two extract abstract operations, but
- this is a bit weird on instances of CoutQueuingStrategy or ByteLengthQueuingStrategy, and we'd want a more Rust-like approach that keeps the highWaterMark and the size function packaged together
*/
// TODO put it in default

pub struct QueuingStrategyInit {
    /// Note that the provided high water mark will not be validated ahead of time. Instead, if it is negative, NaN, or not a number, the resulting CountQueuingStrategy will cause the corresponding stream constructor to throw.
    pub high_water_mark: idl::UnrestrictedDouble,
}

impl TryFromJs for QueuingStrategyInit {
    fn try_from_js(value: &JsValue, context: &mut Context<'_>) -> JsResult<Self> {
        let this = value.to_object(context)?;
        let high_water_mark: idl::UnrestrictedDouble =
            get_JsObject_property(&this, "highWaterMark", context)?
                .try_js_into::<Option<idl::UnrestrictedDouble>>(context)?
                .expect("Missing highWaterMark property");

        Ok(QueuingStrategyInit { high_water_mark })
    }
}

pub enum QueuingStrategy {
    Count(CountQueuingStrategy),
    ByteLength(Todo),
    Other(OtherQueuingStrategy),
}
/*
impl QueuingStrategyTrait for OtherQueuingStrategy {
    fn high_water_mark(&self) -> Option<idl::UnrestrictedDouble> {
        self.high_water_mark
    }

    fn size(
        &self,
        chunk: Chunk,
        context: &mut Context,
    ) -> JsResult<idl::UnrestrictedDouble> {
        // This is implemented as an η-expanded version of the spec's `ExtractSizeAlgorithm` (modulo the extra `context` argument being passed around):
        // `strategy.size(chunk, context)` corresponds to `ExtractSizeAlgorithm(strategy)(chunk)`
        // https://streams.spec.whatwg.org/#make-size-algorithm-from-size-function
        match self.size {
            None => Ok(1.0),
            Some(ref size) => size.call(JsUndefined::Undefined, (chunk,), context),
        }
    }
} */

/*





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
pub struct CountQueuingStrategy {
    pub high_water_mark: idl::UnrestrictedDouble,
}

pub trait QueuingStrategyTrait {
    /// [Streams Standard - § 7.1.][https://streams.spec.whatwg.org/#qs-api]
    /// > **`highWaterMark`, of type unrestricted double**
    /// >
    /// > A non-negative number indicating the high water mark of the stream using this queuing strategy.
    fn high_water_mark(&self) -> Option<idl::UnrestrictedDouble>;

    /// [Streams Standard - § 7.1.][https://streams.spec.whatwg.org/#qs-api]
    /// > **`size(chunk)` (non-byte streams only), of type QueuingStrategySize**
    /// >
    /// > A function that computes and returns the finite non-negative size of the given chunk value.
    /// >
    /// > The result is used to determine backpressure, manifesting via the appropriate desiredSize property: either defaultController.desiredSize, byteController.desiredSize, or writer.desiredSize, depending on where the queuing strategy is being used. For readable streams, it also governs when the underlying source's pull() method is called.
    /// >
    /// > This function has to be idempotent and not cause side effects; very strange results can occur otherwise.
    /// >
    /// > For readable byte streams, this function is not used, as chunks are always measured in bytes.
    ///
    /// Semantically, we should have
    /// `size : Option<Fn (&Self, Chunk, &mut Context) -> JsResult<idl::UnrestrictedDouble>>`
    ///
    fn size(
        &self,
        chunk: Chunk,
        context: &mut Context,
    ) -> Option<JsResult<idl::UnrestrictedDouble>>;
}

#[derive(TryFromJs)]


/// [Streams Standard - § 7.1.][https://streams.spec.whatwg.org/#qs-api]
/// > ```
/// > dictionary QueuingStrategyInit {
/// >   required unrestricted double highWaterMark;
/// > };
/// > ```
pub struct CountQueuingStrategy {
    high_water_mark: idl::UnrestrictedDouble,
}

pub trait QueuingStrategyAbstractOperations: QueuingStrategyTrait {
    /// [Streams Standard - § 7.4.][https://streams.spec.whatwg.org/#qs-abstract-ops]
    /// > `ExtractHighWaterMark(strategy, defaultHWM)`
    /// >
    /// > Note: +∞ is explicitly allowed as a valid high water mark. It causes backpressure to never be applied.
    fn extrat_high_water_mark(
        &self,
        default_hwm: idl::UnrestrictedDouble,
    ) -> JsResult<idl::UnrestrictedDouble> {
        // 1. If strategy["highWaterMark"] does not exist, return defaultHWM.
        // 2. Let highWaterMark be strategy["highWaterMark"].
        let Some(high_water_mark) = self.high_water_mark() else {
            return Ok(default_hwm);
        };
        // 3. If highWaterMark is NaN or highWaterMark < 0, throw a RangeError exception.
        if high_water_mark.is_nan() || high_water_mark < 0.0 {
            return Err(JsNativeError::range()
                .with_message(format!("Invalid highWaterMark: {}", high_water_mark))
                .into());
        }
        // 4. Return highWaterMark.
        return Ok(high_water_mark);
    }
}

impl<T: QueuingStrategyTrait> QueuingStrategyAbstractOperations for T {}
 */

pub struct QueuingStrategyApi;

impl jstz_core::Api for QueuingStrategyApi {
    fn init(self, context: &mut Context<'_>) {
        register_global_class::<CountQueuingStrategyClass>(context)
            .expect("The `CountQueuingStrategy` class shouldn't exist yet")
    }
}
