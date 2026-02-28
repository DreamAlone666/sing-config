pub mod outbound;

pub use outbound::{Outbound, OutboundKind, Selector, UrlTest};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub outbounds: Vec<Outbound>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
