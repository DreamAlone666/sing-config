pub mod outbound;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use outbound::Outbound;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    outbounds: Vec<Outbound>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}
