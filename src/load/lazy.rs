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
}

impl LoadProvider for LazyLoader {
    type Error = LoadProviderError;

    fn load_provider(&self, tag: &str) -> Result<&sing_box::Config, Self::Error> {
        let (provider, cell) = self.map.get(tag).ok_or(LoadProviderError::NotFound)?;
        cell.get_or_try_init(|| match provider {
            Provider::Path(path) => {
                let file = File::open(path)?;
                let config = serde_json::from_reader(file)?;
                Ok(config)
            }
            Provider::Url(_url) => todo!(),
        })
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
}
