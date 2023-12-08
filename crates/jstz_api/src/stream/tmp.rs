//! Temporary definitions to allow compiling before defining all types

use boa_engine::{
    js_string, property::PropertyKey, Context, JsObject, JsResult, JsValue,
};

use crate::todo::Todo;

pub type ReadableStreamDefaultController = Todo;
pub type ReadableByteStreamController = Todo;

// TODO check that this function works as intended in all cases,
// and move it either to a new derive macro for TryFromJs, or to JsObject
#[allow(non_snake_case)]
pub fn get_JsObject_property(
    obj: &JsObject,
    name: &str,
    context: &mut Context<'_>,
) -> JsResult<JsValue> {
    let key = PropertyKey::from(js_string!(name));
    let key2 = key.clone();
    let has_prop = obj.has_property(key, context)?;
    if has_prop {
        obj.get(key2, context)
    } else {
        Ok(JsValue::Undefined)
    }
}
