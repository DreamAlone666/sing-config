use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::{
    load::LoadProvider,
    sing_box::outbound::{Outbound as SingBoxOutbound, OutboundKind as SingBoxOutboundKind},
    sing_config::outbound::{
        Outbound as SingConfigOutbound, OutboundKind as SingConfigOutboundKind,
    },
};

#[derive(Debug, Error)]
#[error("未能转换出站 `{tag}`")]
pub struct ConvertOutboundsError<E> {
    tag: String,
    source: ConvertOutboundsErrorSource<E>,
}

impl<E> ConvertOutboundsError<E> {
    fn new(tag: String, source: ConvertOutboundsErrorSource<E>) -> Self {
        Self { tag, source }
    }
}

#[derive(Debug, Error)]
enum ConvertOutboundsErrorSource<E> {
    #[error("未能加载 provider `{tag}`")]
    LoadProvider { tag: String, source: E },
    #[error("尝试合并该出站时，发现标签已经被其他出站占用")]
    InnerTagConflict,
    #[error("尝试合并来自 provider `{from}` 的出站 `{tag}` 时，发现标签已经被其他出站占用")]
    ProviderTagConflict { tag: String, from: String },
}

/// 辅助类。
struct Converter<'a, L> {
    output: HashMap<String, SingBoxOutbound>,
    merged_providers: HashSet<String>,
    loader: &'a L,
}

impl<'a, L: LoadProvider> Converter<'a, L> {
    fn new(loader: &'a L) -> Self {
        Self {
            output: HashMap::new(),
            merged_providers: HashSet::new(),
            loader,
        }
    }

    /// 尝试合并 provider，并返回出站列表。
    fn merge_provider(
        &mut self,
        tag: impl Into<String>,
    ) -> Result<&[SingBoxOutbound], ConvertOutboundsErrorSource<L::Error>> {
        let tag = tag.into();
        let config = self.loader.load_provider(&tag).map_err(|e| {
            ConvertOutboundsErrorSource::LoadProvider {
                tag: tag.clone(),
                source: e,
            }
        })?;

        if !self.merged_providers.contains(&tag) {
            // provider 未合并，先逐一检查标签冲突
            self.output.reserve(config.outbounds.len());
            for outbound in &config.outbounds {
                if self.output.contains_key(&outbound.tag) {
                    return Err(ConvertOutboundsErrorSource::ProviderTagConflict {
                        tag: outbound.tag.clone(),
                        from: tag,
                    });
                }
                self.output.insert(outbound.tag.clone(), outbound.clone());
            }
            self.merged_providers.insert(tag);
        }

        Ok(&config.outbounds)
    }

    /// 将 `outbound_providers` 展开到 `outbounds` 中。
    fn expand_providers(
        &mut self,
        outbounds: &mut Vec<String>,
        outbound_providers: Vec<String>,
    ) -> Result<(), ConvertOutboundsErrorSource<L::Error>> {
        for tag in outbound_providers {
            let provider_outbounds = self.merge_provider(tag)?;
            outbounds.extend(provider_outbounds.iter().map(|x| x.tag.clone()));
        }
        Ok(())
    }

    /// 尝试合并出站。
    fn merge_outbound(
        &mut self,
        tag: impl Into<String>,
        kind: SingBoxOutboundKind,
    ) -> Result<(), ConvertOutboundsError<L::Error>> {
        let tag = tag.into();
        if self.output.contains_key(&tag) {
            return Err(ConvertOutboundsError::new(
                tag,
                ConvertOutboundsErrorSource::InnerTagConflict,
            ));
        }
        self.output
            .insert(tag.clone(), SingBoxOutbound::new(tag, kind));
        Ok(())
    }
}

/// 将 sing-config 的出站列表转换成 sing-box 的出站列表，使用外部提供的加载器来加载 provider。
///
/// 返回一个出站标签到出站的映射，确保了出站标签不存在冲突。
pub fn convert_outbounds<L: LoadProvider>(
    input: Vec<SingConfigOutbound>,
    loader: &L,
) -> Result<HashMap<String, SingBoxOutbound>, ConvertOutboundsError<L::Error>> {
    let mut converter = Converter::new(loader);

    for SingConfigOutbound { tag, kind } in input {
        let handled_kind = match kind {
            SingConfigOutboundKind::Selector(selector) => {
                let (mut selector, outbound_providers) = selector.split();
                converter
                    .expand_providers(&mut selector.outbounds, outbound_providers)
                    .map_err(|e| ConvertOutboundsError::new(tag.clone(), e))?;
                SingBoxOutboundKind::Selector(selector)
            }
            SingConfigOutboundKind::UrlTest(url_test) => {
                let (mut url_test, outbound_providers) = url_test.split();
                converter
                    .expand_providers(&mut url_test.outbounds, outbound_providers)
                    .map_err(|e| ConvertOutboundsError::new(tag.clone(), e))?;
                SingBoxOutboundKind::UrlTest(url_test)
            }
            SingConfigOutboundKind::Unknown(map) => SingBoxOutboundKind::Unknown(map),
        };

        // 转换完后，也尝试将自己合并进去
        converter.merge_outbound(tag, handled_kind)?;
    }

    Ok(converter.output)
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::{Map, Value};

    use super::*;
    use crate::{
        sing_box,
        sing_config::outbound::{Selector as SingConfigSelector, UrlTest as SingConfigUrlTest},
    };

    struct MockLoader {
        map: HashMap<String, sing_box::Config>,
    }

    impl MockLoader {
        fn new() -> Self {
            Self {
                map: HashMap::new(),
            }
        }

        fn add_provider(&mut self, tag: impl Into<String>, outbounds: Vec<SingBoxOutbound>) {
            self.map.insert(
                tag.into(),
                sing_box::Config {
                    outbounds,
                    extra: Map::new(),
                },
            );
        }
    }

    impl LoadProvider for MockLoader {
        type Error = LoadProviderError;

        fn load_provider(&self, tag: &str) -> Result<&sing_box::Config, Self::Error> {
            self.map.get(tag).ok_or(LoadProviderError)
        }
    }

    #[derive(Debug, Error)]
    #[error("provider 不存在")]
    struct LoadProviderError;

    fn sing_box_unknown_outbound(
        tag: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> SingBoxOutbound {
        SingBoxOutbound::new(
            tag.into(),
            sing_box::outbound::OutboundKind::Unknown(Map::from_iter([(
                key.into(),
                Value::String(value.into()),
            )])),
        )
    }

    #[test]
    fn expand_selector() -> Result<(), Box<dyn Error>> {
        let mut loader = MockLoader::new();
        let provider_outbound_1 = sing_box_unknown_outbound("p1_out1", "foo", "bar");
        let provider_outbound_2 = sing_box_unknown_outbound("p1_out2", "bar", "foo");
        loader.add_provider(
            "provider1",
            vec![provider_outbound_1.clone(), provider_outbound_2.clone()],
        );

        let selector = SingConfigSelector {
            outbounds: vec!["existing_outbound".to_string()],
            outbound_providers: vec!["provider1".to_string()],
            extra: Map::new(),
        };
        let input = vec![SingConfigOutbound {
            tag: "selector1".to_string(),
            kind: SingConfigOutboundKind::Selector(selector),
        }];
        let output = convert_outbounds(input, &loader)?;

        // Verify "selector1" is present and expanded
        match &output["selector1"].kind {
            SingBoxOutboundKind::Selector(s) => assert_eq!(
                s.outbounds.as_slice(),
                &[
                    "existing_outbound",
                    &provider_outbound_1.tag,
                    &provider_outbound_2.tag
                ]
            ),
            _ => panic!("selector1 should be a selector"),
        }

        // Verify provider outbounds are present and match exactly
        assert_eq!(
            output.get(&provider_outbound_1.tag),
            Some(&provider_outbound_1)
        );
        assert_eq!(
            output.get(&provider_outbound_2.tag),
            Some(&provider_outbound_2)
        );

        Ok(())
    }

    #[test]
    fn unknown_pass_through() -> Result<(), Box<dyn Error>> {
        let loader = MockLoader::new();

        let unknown_map = Map::from_iter([
            ("foo".to_string(), Value::String("bar".into())),
            ("bar".to_string(), Value::String("foo".into())),
        ]);
        let input = vec![SingConfigOutbound {
            tag: "unknown1".to_string(),
            kind: SingConfigOutboundKind::Unknown(unknown_map.clone()),
        }];
        let output = convert_outbounds(input, &loader)?;

        let expected = SingBoxOutbound {
            tag: "unknown1".to_string(),
            kind: SingBoxOutboundKind::Unknown(unknown_map),
        };
        assert_eq!(output.get("unknown1"), Some(&expected));

        Ok(())
    }

    #[test]
    fn inner_tag_conflict() {
        let loader = MockLoader::new();
        let input = vec![
            SingConfigOutbound {
                tag: "tag1".to_string(),
                kind: SingConfigOutboundKind::Unknown(Map::new()),
            },
            SingConfigOutbound {
                tag: "tag1".to_string(),
                kind: SingConfigOutboundKind::Unknown(Map::new()),
            },
        ];

        let err = convert_outbounds(input, &loader).unwrap_err();
        assert_eq!(err.tag, "tag1");
        assert!(matches!(
            err.source,
            ConvertOutboundsErrorSource::InnerTagConflict
        ));
    }

    #[test]
    fn provider_tag_conflict() {
        let mut loader = MockLoader::new();
        let provider_outbound = sing_box_unknown_outbound("conflict_tag", "foo", "bar");
        loader.add_provider("provider1", vec![provider_outbound]);

        let input = vec![
            SingConfigOutbound {
                tag: "conflict_tag".to_string(),
                kind: SingConfigOutboundKind::Unknown(Map::new()),
            },
            SingConfigOutbound {
                tag: "selector1".to_string(),
                kind: SingConfigOutboundKind::Selector(SingConfigSelector {
                    outbounds: vec![],
                    outbound_providers: vec!["provider1".to_string()],
                    extra: Map::new(),
                }),
            },
        ];

        let err = convert_outbounds(input, &loader).unwrap_err();
        assert_eq!(err.tag, "selector1");
        match err.source {
            ConvertOutboundsErrorSource::ProviderTagConflict { tag, from } => {
                assert_eq!(tag, "conflict_tag");
                assert_eq!(from, "provider1");
            }
            _ => panic!("Expected ProviderTagConflict, got {:?}", err),
        }
    }

    #[test]
    fn expand_url_test() -> Result<(), Box<dyn Error>> {
        let mut loader = MockLoader::new();
        let provider_outbound_1 = sing_box_unknown_outbound("p1_out1", "foo", "bar");
        let provider_outbound_2 = sing_box_unknown_outbound("p1_out2", "bar", "foo");
        loader.add_provider(
            "provider1",
            vec![provider_outbound_1.clone(), provider_outbound_2.clone()],
        );

        let url_test = SingConfigUrlTest {
            outbounds: vec!["existing_outbound".to_string()],
            outbound_providers: vec!["provider1".to_string()],
            extra: Map::new(),
        };
        let input = vec![SingConfigOutbound {
            tag: "url_test1".to_string(),
            kind: SingConfigOutboundKind::UrlTest(url_test),
        }];
        let output = convert_outbounds(input, &loader)?;

        // Verify "url_test1" is present and expanded
        match &output["url_test1"].kind {
            SingBoxOutboundKind::UrlTest(u) => assert_eq!(
                u.outbounds.as_slice(),
                &[
                    "existing_outbound",
                    &provider_outbound_1.tag,
                    &provider_outbound_2.tag
                ]
            ),
            _ => panic!("url_test1 should be a UrlTest"),
        }

        // Verify provider outbounds are present and match exactly
        assert_eq!(
            output.get(&provider_outbound_1.tag),
            Some(&provider_outbound_1)
        );
        assert_eq!(
            output.get(&provider_outbound_2.tag),
            Some(&provider_outbound_2)
        );

        Ok(())
    }
}
