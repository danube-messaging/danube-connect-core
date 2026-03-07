use crate::config::{ConsumerConfig, ProducerConfig, SchemaConfig, SubscriptionType};

#[derive(Debug, Clone)]
pub struct RouteSchemaPolicy {
    pub expected_subject: Option<String>,
    pub output_schema: Option<SchemaConfig>,
}

impl RouteSchemaPolicy {
    pub fn none() -> Self {
        Self {
            expected_subject: None,
            output_schema: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_sink_route_roundtrip() {
        let config = ConsumerConfig {
            topic: "/default/events".to_string(),
            consumer_name: "events-consumer".to_string(),
            subscription: "events-sub".to_string(),
            subscription_type: SubscriptionType::Shared,
            expected_schema_subject: Some("events-value".to_string()),
        };

        let route = config.route();
        assert_eq!(route.topic, "/default/events");
        assert_eq!(route.subscription.consumer_name, "events-consumer");
        assert_eq!(route.subscription.subscription, "events-sub");
        assert!(matches!(
            route.subscription.subscription_type,
            SubscriptionType::Shared
        ));
        assert_eq!(
            route.schema.expected_subject.as_deref(),
            Some("events-value")
        );
        assert!(route.schema.output_schema.is_none());

        let roundtrip = ConsumerConfig::from_route(route);
        assert_eq!(roundtrip.topic, config.topic);
        assert_eq!(roundtrip.consumer_name, config.consumer_name);
        assert_eq!(roundtrip.subscription, config.subscription);
        assert!(matches!(
            roundtrip.subscription_type,
            SubscriptionType::Shared
        ));
        assert_eq!(
            roundtrip.expected_schema_subject,
            Some("events-value".to_string())
        );
    }

    #[test]
    fn test_source_route_roundtrip() {
        let config = ProducerConfig {
            topic: "/default/output".to_string(),
            partitions: 4,
            reliable_dispatch: true,
            schema_config: Some(SchemaConfig {
                subject: "output-value".to_string(),
                schema_type: "json_schema".to_string(),
                schema_file: PathBuf::from("schemas/output.json"),
                auto_register: true,
                version_strategy: crate::VersionStrategy::Pinned(2),
            }),
        };

        let route = config.route();
        assert_eq!(route.topic, "/default/output");
        assert_eq!(route.dispatch.partitions, 4);
        assert!(route.dispatch.reliable_dispatch);
        assert!(route.schema.expected_subject.is_none());
        assert!(route.schema.output_schema.is_some());

        let roundtrip = ProducerConfig::from_route(route);
        assert_eq!(roundtrip.topic, config.topic);
        assert_eq!(roundtrip.partitions, config.partitions);
        assert_eq!(roundtrip.reliable_dispatch, config.reliable_dispatch);
        assert!(roundtrip.schema_config.is_some());
        assert_eq!(
            roundtrip
                .schema_config
                .as_ref()
                .map(|schema| schema.subject.as_str()),
            Some("output-value")
        );
    }
}

impl From<ConsumerConfig> for SinkRoute {
    fn from(config: ConsumerConfig) -> Self {
        Self {
            topic: config.topic,
            subscription: RouteSubscriptionPolicy {
                consumer_name: config.consumer_name,
                subscription: config.subscription,
                subscription_type: config.subscription_type,
            },
            schema: RouteSchemaPolicy {
                expected_subject: config.expected_schema_subject,
                output_schema: None,
            },
        }
    }
}

impl From<SinkRoute> for ConsumerConfig {
    fn from(route: SinkRoute) -> Self {
        Self {
            topic: route.topic,
            consumer_name: route.subscription.consumer_name,
            subscription: route.subscription.subscription,
            subscription_type: route.subscription.subscription_type,
            expected_schema_subject: route.schema.expected_subject,
        }
    }
}

impl From<ProducerConfig> for SourceRoute {
    fn from(config: ProducerConfig) -> Self {
        Self {
            topic: config.topic,
            dispatch: RouteDispatchPolicy {
                partitions: config.partitions,
                reliable_dispatch: config.reliable_dispatch,
            },
            schema: RouteSchemaPolicy {
                expected_subject: None,
                output_schema: config.schema_config,
            },
        }
    }
}

impl From<SourceRoute> for ProducerConfig {
    fn from(route: SourceRoute) -> Self {
        Self {
            topic: route.topic,
            partitions: route.dispatch.partitions,
            reliable_dispatch: route.dispatch.reliable_dispatch,
            schema_config: route.schema.output_schema,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RouteSubscriptionPolicy {
    pub consumer_name: String,
    pub subscription: String,
    pub subscription_type: SubscriptionType,
}

impl RouteSubscriptionPolicy {
    pub fn new(
        consumer_name: impl Into<String>,
        subscription: impl Into<String>,
        subscription_type: SubscriptionType,
    ) -> Self {
        Self {
            consumer_name: consumer_name.into(),
            subscription: subscription.into(),
            subscription_type,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RouteDispatchPolicy {
    pub partitions: usize,
    pub reliable_dispatch: bool,
}

impl RouteDispatchPolicy {
    pub fn new(partitions: usize, reliable_dispatch: bool) -> Self {
        Self {
            partitions,
            reliable_dispatch,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SinkRoute {
    pub topic: String,
    pub subscription: RouteSubscriptionPolicy,
    pub schema: RouteSchemaPolicy,
}

impl SinkRoute {
    pub fn new(
        topic: impl Into<String>,
        subscription: RouteSubscriptionPolicy,
        schema: RouteSchemaPolicy,
    ) -> Self {
        Self {
            topic: topic.into(),
            subscription,
            schema,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceRoute {
    pub topic: String,
    pub dispatch: RouteDispatchPolicy,
    pub schema: RouteSchemaPolicy,
}

impl SourceRoute {
    pub fn new(
        topic: impl Into<String>,
        dispatch: RouteDispatchPolicy,
        schema: RouteSchemaPolicy,
    ) -> Self {
        Self {
            topic: topic.into(),
            dispatch,
            schema,
        }
    }
}
