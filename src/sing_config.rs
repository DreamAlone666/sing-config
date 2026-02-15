pub mod outbound;
pub mod provider;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use outbound::Outbound;
use provider::Provider;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub providers: HashMap<String, Provider>,
    #[serde(default)]
    pub outbounds: Vec<Outbound>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
