use indexmap::{IndexMap, map::Entry};
use thiserror::Error;

use crate::{
    load::LoadProvider,
    sing_box::{
        self,
        outbound::{Outbound as SingBoxOutbound, OutboundKind as SingBoxOutboundKind},
    },
    sing_config::outbound::{
        Outbound as SingConfigOutbound, OutboundKind as SingConfigOutboundKind,
    },
};

#[derive(Debug, Error)]
pub enum ConvertOutboundsError<E> {
    #[error("未能加载 provider `{tag}`")]
    LoadProvider { tag: String, source: E },
    #[error("输入的节点中，存在两个标签为 `{tag}` 的出站")]
    InputTagConflict { tag: String },
    #[error("尝试合并来自 provider `{from}` 的出站 `{tag}` 时，发现标签已经被其他出站占用")]
    ProviderTagConflict { tag: String, from: String },
}

/// 将 sing-config 的出站列表转换成 sing-box 的出站列表，使用外部提供的加载器来加载 provider。
///
/// 返回一个出站标签到出站的映射，确保了出站标签不存在冲突。
pub fn convert_outbounds<L: LoadProvider>(
    input: Vec<SingConfigOutbound>,
    loader: &L,
) -> Result<IndexMap<String, SingBoxOutbound>, ConvertOutboundsError<L::Error>> {
    let (mut outbound_map, provider_consumers) = convert_input(input)?;
    for (provider_tag, consumers) in provider_consumers {
        let config = loader.load_provider(&provider_tag).map_err(|source| {
            ConvertOutboundsError::LoadProvider {
                tag: provider_tag.clone(),
                source,
            }
        })?;
        merge_provider(&mut outbound_map, config).map_err(|tag| {
            ConvertOutboundsError::ProviderTagConflict {
                tag: tag.to_string(),
                from: provider_tag,
            }
        })?;
        for consumer in consumers {
            consume_provider(&mut outbound_map[&consumer], config);
        }
    }

    Ok(outbound_map)
}

// 辅助类型别名
type OutboundMap = IndexMap<String, SingBoxOutbound>;
type ProviderConsumers = IndexMap<String, Vec<String>>;

/// 将输入出站列表转换为标签到输出出站的映射，并收集其中的 `providers` 需求组成另一个 provider 标签到出站标签的映射。
fn convert_input<E>(
    input: Vec<SingConfigOutbound>,
) -> Result<(OutboundMap, ProviderConsumers), ConvertOutboundsError<E>> {
    let mut outbound_map = IndexMap::with_capacity(input.len());
    let mut provider_consumers: ProviderConsumers = IndexMap::new();
    for SingConfigOutbound { tag, kind } in input {
        match outbound_map.entry(tag.clone()) {
            Entry::Occupied(_) => {
                return Err(ConvertOutboundsError::InputTagConflict { tag });
            }
            Entry::Vacant(entry) => {
                let (handled_kind, provider_tags) = match kind {
                    SingConfigOutboundKind::Selector(selector) => {
                        let (selector, providers) = selector.split();
                        (selector.into(), Some(providers))
                    }
                    SingConfigOutboundKind::UrlTest(url_test) => {
                        let (url_test, providers) = url_test.split();
                        (url_test.into(), Some(providers))
                    }
                    SingConfigOutboundKind::Unknown(unknown) => {
                        (SingBoxOutboundKind::Unknown(unknown), None)
                    }
                };

                if let Some(provider_tags) = provider_tags {
                    for provider_tag in provider_tags {
                        provider_consumers
                            .entry(provider_tag)
                            .or_default()
                            .push(tag.clone());
                    }
                }
                entry.insert(SingBoxOutbound::new(tag, handled_kind));
            }
        }
    }

    Ok((outbound_map, provider_consumers))
}

/// 尝试合并来自 providers 的节点。
///
/// # Errors
///
/// 如果节点的标签被占用，则将其标签作为错误抛出。
fn merge_provider<'a>(map: &mut OutboundMap, config: &'a sing_box::Config) -> Result<(), &'a str> {
    map.reserve(config.outbounds.len());
    for outbound in &config.outbounds {
        match map.entry(outbound.tag.clone()) {
            Entry::Occupied(_) => return Err(&outbound.tag),
            Entry::Vacant(entry) => {
                entry.insert(outbound.clone());
            }
        }
    }
    Ok(())
}

/// 将 provider 的出站标签展开到出站的出站列表中。
///
/// # Panics
///
/// 如果出站的类型不正确，函数会崩溃。
fn consume_provider(outbound: &mut SingBoxOutbound, config: &sing_box::Config) {
    let tags = config.outbounds.iter().map(|out| out.tag.clone());
    match &mut outbound.kind {
        SingBoxOutboundKind::Selector(selector) => selector.outbounds.extend(tags),
        SingBoxOutboundKind::UrlTest(url_test) => url_test.outbounds.extend(tags),
        SingBoxOutboundKind::Unknown(_) => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, error::Error};

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
        assert!(matches!(
            err,
            ConvertOutboundsError::InputTagConflict { tag } if tag == "tag1"
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
        match err {
            ConvertOutboundsError::ProviderTagConflict { tag, from } => {
                assert_eq!(tag, "conflict_tag");
                assert_eq!(from, "provider1");
            }
            _ => panic!("Expected ProviderTagConflict, got {:?}", err),
        }
    }

    #[test]
    fn keep_input_order_and_inputs_are_first() -> Result<(), Box<dyn Error>> {
        let mut loader = MockLoader::new();
        let provider_outbound_1 = sing_box_unknown_outbound("p1_out1", "foo", "bar");
        let provider_outbound_2 = sing_box_unknown_outbound("p1_out2", "bar", "foo");
        loader.add_provider(
            "provider1",
            vec![provider_outbound_1.clone(), provider_outbound_2.clone()],
        );

        let input = vec![
            SingConfigOutbound {
                tag: "first_selector".to_string(),
                kind: SingConfigOutboundKind::Selector(SingConfigSelector {
                    outbounds: vec!["direct_1".to_string()],
                    outbound_providers: vec!["provider1".to_string()],
                    extra: Map::new(),
                }),
            },
            SingConfigOutbound {
                tag: "second_unknown".to_string(),
                kind: SingConfigOutboundKind::Unknown(Map::new()),
            },
            SingConfigOutbound {
                tag: "third_url_test".to_string(),
                kind: SingConfigOutboundKind::UrlTest(SingConfigUrlTest {
                    outbounds: vec!["direct_2".to_string()],
                    outbound_providers: vec![],
                    extra: Map::new(),
                }),
            },
        ];

        let output = convert_outbounds(input, &loader)?;
        let output_tags: Vec<&str> = output.keys().map(String::as_str).collect();
        assert_eq!(
            output_tags,
            vec![
                "first_selector",
                "second_unknown",
                "third_url_test",
                "p1_out1",
                "p1_out2"
            ]
        );

        Ok(())
    }
}
