use derive_more::From;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, From)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OutboundKind {
    Selector(Selector),
    UrlTest(UrlTest),
    #[serde(untagged)]
    #[from(skip)]
    Unknown(Map<String, Value>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selector {
    #[serde(default)]
    pub outbounds: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlTest {
    #[serde(default)]
    pub outbounds: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
