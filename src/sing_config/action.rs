use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::sing_box::Outbound;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Include(Filter),
    Exclude(Filter),
}

impl Action {
    /// 对出站列表应用操作，返回操作后的出站列表。
    pub fn apply(&self, outbounds: Vec<Outbound>) -> Result<Vec<Outbound>, ApplyActionError> {
        let outbounds = match self {
            Self::Include(filter) => {
                let compiled = filter.compile()?;
                outbounds
                    .into_iter()
                    .filter(|outbound| compiled.is_match(outbound))
                    .collect()
            }
            Self::Exclude(filter) => {
                let compiled = filter.compile()?;
                outbounds
                    .into_iter()
                    .filter(|outbound| !compiled.is_match(outbound))
                    .collect()
            }
        };

        Ok(outbounds)
    }
}

#[derive(Debug, Error)]
#[error("未能编译 regex")]
pub struct ApplyActionError(#[from] regex::Error);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filter {
    pub field: OutboundField,
    pub regex: String,
}

impl Filter {
    /// 编译过滤器以使用其功能。
    ///
    /// # Errors
    ///
    /// 如果 regex 编译失败会报错。
    fn compile(&self) -> Result<CompiledFilter<'_>, regex::Error> {
        let regex = Regex::new(&self.regex)?;
        Ok(CompiledFilter::new(self, regex))
    }
}

/// 编译好的过滤器，可以对出站进行过滤。
struct CompiledFilter<'a> {
    filter: &'a Filter,
    regex: Regex,
}

impl<'a> CompiledFilter<'a> {
    fn new(filter: &'a Filter, regex: Regex) -> Self {
        Self { filter, regex }
    }

    /// 如果节点和过滤器匹配则返回 `true`。
    fn is_match(&self, outbound: &Outbound) -> bool {
        let value = match self.filter.field {
            // 没有类型名的出站使用空字符让 regex 忽略它
            OutboundField::Type => outbound.kind.type_name().unwrap_or(""),
            OutboundField::Tag => &outbound.tag,
        };
        self.regex.is_match(value)
    }
}

/// 受实现方式的限制，过滤器目前只支持这些出站字段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboundField {
    Type,
    Tag,
}
