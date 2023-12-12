use boa_engine::{Context, JsResult};
use boa_gc::{Finalize, Trace};
use jstz_core::{
    js_fn::{JsCallable, JsCallableWithoutThis, JsFn},
    value::JsUndefined,
};

use crate::{idl, stream::Chunk};

use super::{
    high_water_mark::ExtractHighWaterMark, HighWaterMarkAndSizeAlgorithm,
    QueuingStrategyKind,
};

pub trait SizeAlgorithm {
    fn call(
        &self,
        chunk: &Chunk,
        context: &mut Context<'_>,
    ) -> JsResult<idl::UnrestrictedDouble>;
}

#[derive(Default, Finalize, Trace)]
pub enum CountQueuingStrategySizeAlgorithm {
    #[default]
    ReturnOne,
}

impl SizeAlgorithm for CountQueuingStrategySizeAlgorithm {
    fn call(
        &self,
        _chunk: &Chunk,
        _context: &mut Context<'_>,
    ) -> JsResult<idl::UnrestrictedDouble> {
        Ok(1.0)
    }
}

#[derive(Default, Finalize, Trace)]
pub enum ByteLengthQueuingStrategySizeAlgorithm {
    #[default]
    ReturnByteLengthOfChunk,
}

impl SizeAlgorithm for ByteLengthQueuingStrategySizeAlgorithm {
    fn call(
        &self,
        chunk: &Chunk,
        context: &mut boa_engine::prelude::Context<'_>,
    ) -> boa_engine::prelude::JsResult<idl::UnrestrictedDouble> {
        todo!()
    }
}

/// [Streams Standard - § 7.1.][https://streams.spec.whatwg.org/#qs-api]
/// > `callback QueuingStrategySize = unrestricted double (any chunk);`
pub type CustomQueuingStrategySizeAlgorithm =
    JsFn<JsUndefined, 1, (idl::Any,), idl::UnrestrictedDouble>;

impl SizeAlgorithm for CustomQueuingStrategySizeAlgorithm {
    fn call(
        &self,
        chunk: &Chunk,
        context: &mut Context<'_>,
    ) -> JsResult<idl::UnrestrictedDouble> {
        // TODO check that this use of to_owned() is fine
        JsCallable::call(self, JsUndefined::Undefined, (chunk.to_owned(),), context)
    }
}

pub type QueuingStrategySizeAlgorithm = QueuingStrategyKind<
    CountQueuingStrategySizeAlgorithm,
    ByteLengthQueuingStrategySizeAlgorithm,
    CustomQueuingStrategySizeAlgorithm,
>;

impl<A: SizeAlgorithm, B: SizeAlgorithm, C: SizeAlgorithm> SizeAlgorithm
    for QueuingStrategyKind<A, B, C>
{
    fn call(
        &self,
        chunk: &Chunk,
        context: &mut Context<'_>,
    ) -> JsResult<idl::UnrestrictedDouble> {
        match self {
            QueuingStrategyKind::Count(size_algorithm) => {
                size_algorithm.call(chunk, context)
            }
            QueuingStrategyKind::ByteLength(size_algorithm) => {
                size_algorithm.call(chunk, context)
            }
            QueuingStrategyKind::Custom(size_algorithm) => {
                size_algorithm.call(chunk, context)
            }
        }
    }
}

impl<T: SizeAlgorithm> SizeAlgorithm for Option<T> {
    fn call(
        &self,
        chunk: &Chunk,
        context: &mut boa_engine::prelude::Context<'_>,
    ) -> JsResult<idl::UnrestrictedDouble> {
        match self {
            Some(size_algorithm) => size_algorithm.call(chunk, context),
            None => Ok(1.0),
        }
    }
}

pub trait WithSizeAlgorithm {
    fn size(
        &self,
        chunk: &Chunk,
        context: &mut boa_engine::prelude::Context<'_>,
    ) -> JsResult<idl::UnrestrictedDouble>;
}

impl<T, U: SizeAlgorithm> WithSizeAlgorithm for HighWaterMarkAndSizeAlgorithm<T, U> {
    fn size(
        &self,
        chunk: &Chunk,
        context: &mut boa_engine::prelude::Context<'_>,
    ) -> JsResult<idl::UnrestrictedDouble> {
        self.size_algorithm.call(chunk, context)
    }
}

// TODO add ExtractSize?
