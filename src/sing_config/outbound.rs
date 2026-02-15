use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::sing_box;

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
    #[serde(default)]
    pub outbounds: Vec<String>,
    #[serde(default)]
    pub outbound_providers: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Selector {
    /// 拆分成两部分，第一部分是 sing-box 的选择器出站，另一部分是 providers 列表。
    pub(crate) fn split(self) -> (sing_box::outbound::Selector, Vec<String>) {
        (
            sing_box::outbound::Selector {
                outbounds: self.outbounds,
                extra: self.extra,
            },
            self.outbound_providers,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlTest {
    #[serde(default)]
    pub outbounds: Vec<String>,
    #[serde(default)]
    pub outbound_providers: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl UrlTest {
    /// 拆分成两部分，第一部分是 sing-box 的选择器出站，另一部分是 providers 列表。
    pub(crate) fn split(self) -> (sing_box::outbound::UrlTest, Vec<String>) {
        (
            sing_box::outbound::UrlTest {
                outbounds: self.outbounds,
                extra: self.extra,
            },
            self.outbound_providers,
        )
    }
}
