use std::{collections::HashMap, fs::File, io};

use once_cell::unsync::OnceCell;
use thiserror::Error;

use super::LoadProvider;
use crate::{sing_box, sing_config::provider::Provider};

pub struct LazyLoader {
    map: HashMap<String, (Provider, OnceCell<sing_box::Config>)>,
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

    fn load(&self, tag: &str, is_from_ref: bool) -> Result<&sing_box::Config, LoadProviderError> {
        let (provider, cell) = self.map.get(tag).ok_or(LoadProviderError::NotFound)?;
        cell.get_or_try_init(|| match provider {
            Provider::Path(path) => {
                let file = File::open(path)?;
                let config = serde_json::from_reader(file)?;
                Ok(config)
            }
            Provider::Url(_url) => todo!(),
            Provider::Ref(ref_tag) => {
                if is_from_ref {
                    return Err(LoadProviderError::NestedRef(tag.to_string()));
                }
                if tag == ref_tag {
                    return Err(LoadProviderError::SelfRef);
                }
                self.load(ref_tag, true).cloned()
            }
        })
    }
}

impl LoadProvider for LazyLoader {
    type Error = LoadProviderError;

    fn load_provider(&self, tag: &str) -> Result<&sing_box::Config, Self::Error> {
        self.load(tag, false)
    }
}

#[derive(Debug, Error)]
pub enum LoadProviderError {
    #[error("provider 不存在")]
    NotFound,
    #[error("未能读取文件")]
    ReadFile(#[from] io::Error),
    #[error("未能解析 provider")]
    Parse(#[from] serde_json::Error),
    #[error("不能引用自己")]
    SelfRef,
    #[error("引用了 provider `{0}`，但 `{0}` 不能再进行引用")]
    NestedRef(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_ref_should_error() {
        let providers = HashMap::from([("a".to_string(), Provider::Ref("a".to_string()))]);
        let loader = LazyLoader::new(providers);

        let err = loader.load_provider("a").unwrap_err();
        assert!(matches!(err, LoadProviderError::SelfRef));
    }

    #[test]
    fn nested_ref_should_error() {
        let providers = HashMap::from([
            ("a".to_string(), Provider::Ref("b".to_string())),
            ("b".to_string(), Provider::Ref("c".to_string())),
        ]);
        let loader = LazyLoader::new(providers);

        let err = loader.load_provider("a").unwrap_err();
        assert!(matches!(err, LoadProviderError::NestedRef(tag) if tag == "b"));
    }
}
