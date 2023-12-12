//! [Streams Standard - § 7. Queuing strategies][https://streams.spec.whatwg.org/#qs]

use std::env::home_dir;

use boa_engine::{value::TryFromJs, Context, JsNativeError, JsObject, JsResult, JsValue};
use boa_gc::{Finalize, Trace};
use jstz_core::{
    js_fn::JsFn,
    native::{register_global_class, JsNativeObject},
    value::JsUndefined,
};

use crate::{idl, stream::Chunk, todo::Todo};

use derive_more::*;

use self::{
    builtin::CountQueuingStrategyClass,
    high_water_mark::{ExtractHighWaterMark, HighWaterMark},
    size::{
        ByteLengthQueuingStrategySizeAlgorithm, CountQueuingStrategySizeAlgorithm,
        CustomQueuingStrategySizeAlgorithm, QueuingStrategySizeAlgorithm, SizeAlgorithm,
    },
};

use super::tmp::get_JsObject_property;

pub mod builtin;
pub mod high_water_mark;
pub mod size;

/*
Important for design choice:
- strategies that are not instances of CoutQueuingStrategy or ByteLengthQueuingStrategy may not have a highWaterMark, but this property is never used directly. Instead, the function ExtractHighWaterMark(strategy, defaultHWM) is used, with different defaults depending on the call site. Using ExtractHighWaterMark is not very idiomatic in Rust, but we don't know which default to use when creating the object
- size is provided by a getter attached to the prototype for instances of CoutQueuingStrategy or ByteLengthQueuingStrategy, but can be an arbitrary function for other strategies. We therefore a priori want to keep distinguishing them by having a enum with 3 variants, with a trait implemented by each component, and on the enum by dispatching.

*/
// TODO use it in default for ReadableStream constructor

#[derive(Finalize, Trace)]
pub struct HighWaterMarkAndSizeAlgorithm<T, U> {
    /// [Streams Standard - § 7.1.][https://streams.spec.whatwg.org/#qs-api]
    /// > **`highWaterMark`, of type unrestricted double**
    /// >
    /// > A non-negative number indicating the high water mark of the stream using this queuing strategy.
    pub high_water_mark: T,

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
    pub size_algorithm: U,
}

pub enum QueuingStrategyKind<A, B, C> {
    Count(A),
    ByteLength(B),
    Custom(C),
}

impl<A: TryFromJs, B: TryFromJs, C: TryFromJs> TryFromJs
    for QueuingStrategyKind<A, B, C>
{
    fn try_from_js(value: &JsValue, context: &mut Context<'_>) -> JsResult<Self> {
        A::try_from_js(value, context)
            .map(QueuingStrategyKind::Count)
            .or_else(|_| {
                B::try_from_js(value, context).map(QueuingStrategyKind::ByteLength)
            })
            .or_else(|_| C::try_from_js(value, context).map(QueuingStrategyKind::Custom))
    }
}

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
pub type CountQueuingStrategy = HighWaterMarkAndSizeAlgorithm<
    idl::UnrestrictedDouble,
    CountQueuingStrategySizeAlgorithm,
>;

pub type ByteLengthQueuingStrategy = HighWaterMarkAndSizeAlgorithm<
    idl::UnrestrictedDouble,
    ByteLengthQueuingStrategySizeAlgorithm,
>;

pub type CustomQueuingStrategy = HighWaterMarkAndSizeAlgorithm<
    Option<idl::UnrestrictedDouble>,
    Option<CustomQueuingStrategySizeAlgorithm>,
>;

pub type QueuingStrategyArg = QueuingStrategyKind<
    JsNativeObject<CountQueuingStrategy>,
    JsNativeObject<ByteLengthQueuingStrategy>,
    CustomQueuingStrategy,
>;

pub type QueuingStrategy =
    HighWaterMarkAndSizeAlgorithm<HighWaterMark, QueuingStrategySizeAlgorithm>;

pub struct QueuingStrategyApi;

impl jstz_core::Api for QueuingStrategyApi {
    fn init(self, context: &mut Context<'_>) {
        register_global_class::<CountQueuingStrategyClass>(context)
            .expect("The `CountQueuingStrategy` class shouldn't exist yet")
    }
}
