pub mod action;
pub mod outbound;
pub mod provider;

pub use action::Action;
pub use outbound::{Outbound, OutboundKind, Selector, UrlTest};
pub use provider::{Provider, ProviderKind};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub providers: HashMap<String, Provider>,
    #[serde(default)]
    pub outbounds: Vec<Outbound>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
