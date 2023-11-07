use boa_engine::{js_string, object::ObjectInitializer, property::Attribute};

mod kv;

pub struct DebugApi;

impl DebugApi {
    const NAME: &'static str = "jstz";
}

impl jstz_core::Api for DebugApi {
    fn init(self, context: &mut boa_engine::Context<'_>) {
        let kv_api = kv::KvApi {}.init(context);

        let storage = ObjectInitializer::new(context)
            .property(js_string!("Kv"), kv_api, Attribute::all())
            .build();

        context
            .register_global_property(js_string!(Self::NAME), storage, Attribute::all())
            .expect("The storage object shouldn't exist yet");
    }
}
