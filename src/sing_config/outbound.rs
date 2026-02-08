use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outbound {
    tag: String,
    #[serde(flatten)]
    kind: OutboundKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OutboundKind {
    Selector(Selector),
    #[serde(untagged)]
    Unknown(Map<String, Value>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selector {
    outbounds: Vec<String>,
    outbound_providers: Vec<String>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}
