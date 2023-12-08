use jstz_core::{js_fn::JsFn, value::JsUndefined};

use crate::idl;

/// [Streams Standard - § 7.1.][https://streams.spec.whatwg.org/#qs-api]
/// ```
/// dictionary QueuingStrategy {
///   unrestricted double highWaterMark;
///   QueuingStrategySize size;
/// };
/// ```
pub struct OtherQueuingStrategy {
    pub high_water_mark: Option<idl::UnrestrictedDouble>,
    pub size: Option<OtherQueuingStrategySize>,
}

/// [Streams Standard - § 7.1.][https://streams.spec.whatwg.org/#qs-api]
/// > `callback QueuingStrategySize = unrestricted double (any chunk);`
pub type OtherQueuingStrategySize =
    JsFn<JsUndefined, 1, (idl::Any,), idl::UnrestrictedDouble>;
