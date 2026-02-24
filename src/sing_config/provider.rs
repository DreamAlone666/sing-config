pub mod action;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use action::Action;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Provider {
    #[serde(flatten)]
    pub kind: ProviderKind,
    #[serde(default)]
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Path(PathBuf),
    Url(String),
    /// 引用某个 provider，使用其输出作为当前 provider 的输入。
    ///
    /// 权衡实现和实际使用的复杂度，只允许引用非 `Ref` 类型的 provider。
    Ref(String),
}
