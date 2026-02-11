use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outbound {
    pub tag: String,
    #[serde(flatten)]
    pub kind: OutboundKind,
}

impl Outbound {
    pub fn new(tag: String, kind: OutboundKind) -> Self {
        Self { tag, kind }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OutboundKind {
    Selector(Selector),
    UrlTest(UrlTest),
    #[serde(untagged)]
    Unknown(Map<String, Value>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selector {
    pub outbounds: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlTest {
    pub outbounds: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
