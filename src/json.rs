use serde_json::{Map, Value};

pub type JSON = Value;
pub type Object = Map<String, Value>;
pub type Array = Vec<Value>;
