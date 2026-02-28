use std::{collections::HashMap, fs::File, io};

use once_cell::unsync::OnceCell;
use thiserror::Error;

use super::LoadProvider;
use crate::{
    sing_box::Config,
    sing_config::{Action, Provider, ProviderKind, action::ApplyActionError},
};

pub struct LazyLoader {
    map: HashMap<String, (Provider, OnceCell<Config>)>,
}

impl LazyLoader {
    pub fn new(providers: HashMap<String, Provider>) -> Self {
        Self {
            map: providers
                .into_iter()
                .map(|(tag, provider)| (tag, (provider, OnceCell::new())))
                .collect(),
        }
    }

    fn load(&self, tag: &str, is_from_ref: bool) -> Result<&Config, LoadProviderError> {
        let (Provider { kind, actions }, cell) =
            self.map.get(tag).ok_or(LoadProviderError::NotFound)?;
        cell.get_or_try_init(|| {
            let mut config = match kind {
                ProviderKind::Path(path) => {
                    let file = File::open(path)?;
                    serde_json::from_reader(file)?
                }
                ProviderKind::Url(url) => {
                    let mut response = ureq::get(url).call()?;
                    let reader = response.body_mut().as_reader();
                    serde_json::from_reader(reader)?
                }
                ProviderKind::Ref(ref_tag) => {
                    if is_from_ref {
                        return Err(LoadProviderError::NestedRef(tag.to_string()));
                    }
                    if tag == ref_tag {
                        return Err(LoadProviderError::SelfRef);
                    }
                    self.load(ref_tag, true).cloned()?
                }
            };

            // 按顺序应用操作链
            for action in actions {
                config.outbounds = action.apply(config.outbounds).map_err(|source| {
                    LoadProviderError::ApplyAction {
                        source,
                        action: action.clone(),
                    }
                })?;
            }

            Ok(config)
        })
    }
}

impl LoadProvider for LazyLoader {
    type Error = LoadProviderError;

    fn load_provider(&self, tag: &str) -> Result<&Config, Self::Error> {
        self.load(tag, false)
    }
}

#[derive(Debug, Error)]
pub enum LoadProviderError {
    #[error("provider 不存在")]
    NotFound,
    #[error("未能读取文件")]
    ReadFile(#[from] io::Error),
    #[error("未能解析内容")]
    Parse(#[from] serde_json::Error),
    #[error("不能引用自己")]
    SelfRef,
    #[error("引用了 provider `{0}`，但 `{0}` 不能再进行引用")]
    NestedRef(String),
    #[error("未能应用操作 {action:?}")]
    ApplyAction {
        source: ApplyActionError,
        action: Action,
    },
    #[error("未能完成请求")]
    Request(#[from] ureq::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ref_provider(tag: &str) -> Provider {
        Provider {
            kind: ProviderKind::Ref(tag.to_string()),
            actions: vec![],
        }
    }

    #[test]
    fn self_ref_should_error() {
        let providers = HashMap::from([("a".to_string(), ref_provider("a"))]);
        let loader = LazyLoader::new(providers);

        let err = loader.load_provider("a").unwrap_err();
        assert!(matches!(err, LoadProviderError::SelfRef));
    }

    #[test]
    fn nested_ref_should_error() {
        let providers = HashMap::from([
            ("a".to_string(), ref_provider("b")),
            ("b".to_string(), ref_provider("c")),
        ]);
        let loader = LazyLoader::new(providers);

        let err = loader.load_provider("a").unwrap_err();
        assert!(matches!(err, LoadProviderError::NestedRef(tag) if tag == "b"));
    }
}
