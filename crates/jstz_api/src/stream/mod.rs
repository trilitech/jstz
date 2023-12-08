use boa_engine::Context;

use crate::idl;

use self::{
    queuing_strategy::{
        count::CountQueuingStrategyClass, QueuingStrategy, QueuingStrategyApi,
    },
    readable::ReadableStreamApi,
};

pub mod queuing_strategy;
pub mod readable;
mod tmp;

type Chunk = idl::Any;

pub struct StreamApi;

impl jstz_core::Api for StreamApi {
    fn init(self, context: &mut Context<'_>) {
        ReadableStreamApi.init(context);
        QueuingStrategyApi.init(context);
    }
}
