pub mod outbound;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use outbound::Outbound;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub outbounds: Vec<Outbound>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
