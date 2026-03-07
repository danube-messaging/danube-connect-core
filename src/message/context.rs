use danube_client::SchemaInfo;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct RoutingContext<'a> {
    topic: &'a str,
    key: Option<&'a str>,
    partition: Option<&'a str>,
    attributes: &'a HashMap<String, String>,
}

impl<'a> RoutingContext<'a> {
    pub fn new(
        topic: &'a str,
        key: Option<&'a str>,
        partition: Option<&'a str>,
        attributes: &'a HashMap<String, String>,
    ) -> Self {
        Self {
            topic,
            key,
            partition,
            attributes,
        }
    }

    pub fn topic(&self) -> &'a str {
        self.topic
    }

    pub fn key(&self) -> Option<&'a str> {
        self.key
    }

    pub fn partition(&self) -> Option<&'a str> {
        self.partition
    }

    pub fn attributes(&self) -> &'a HashMap<String, String> {
        self.attributes
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RecordContext<'a> {
    routing: RoutingContext<'a>,
    publish_time: Option<u64>,
    producer_name: Option<&'a str>,
    schema: Option<&'a SchemaInfo>,
}

impl<'a> RecordContext<'a> {
    pub fn new(
        routing: RoutingContext<'a>,
        publish_time: Option<u64>,
        producer_name: Option<&'a str>,
        schema: Option<&'a SchemaInfo>,
    ) -> Self {
        Self {
            routing,
            publish_time,
            producer_name,
            schema,
        }
    }

    pub fn routing(&self) -> RoutingContext<'a> {
        self.routing
    }

    pub fn topic(&self) -> &'a str {
        self.routing.topic()
    }

    pub fn key(&self) -> Option<&'a str> {
        self.routing.key()
    }

    pub fn partition(&self) -> Option<&'a str> {
        self.routing.partition()
    }

    pub fn attributes(&self) -> &'a HashMap<String, String> {
        self.routing.attributes()
    }

    pub fn publish_time(&self) -> Option<u64> {
        self.publish_time
    }

    pub fn producer_name(&self) -> Option<&'a str> {
        self.producer_name
    }

    pub fn schema(&self) -> Option<&'a SchemaInfo> {
        self.schema
    }
}
