use danube_client::SchemaInfo;
use std::collections::HashMap;

/// Routing metadata shared by source and sink records.
#[derive(Debug, Clone, Copy)]
pub struct RoutingContext<'a> {
    topic: &'a str,
    key: Option<&'a str>,
    partition: Option<&'a str>,
    attributes: &'a HashMap<String, String>,
}

impl<'a> RoutingContext<'a> {
    /// Create a routing context from its raw topic, key, partition, and attributes.
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

    /// Return the logical Danube topic associated with the record.
    pub fn topic(&self) -> &'a str {
        self.topic
    }

    /// Return the routing key used for partitioned publishing, if present.
    pub fn key(&self) -> Option<&'a str> {
        self.key
    }

    /// Return the partition identifier for consumed records, if known.
    pub fn partition(&self) -> Option<&'a str> {
        self.partition
    }

    /// Return the user-defined record attributes.
    pub fn attributes(&self) -> &'a HashMap<String, String> {
        self.attributes
    }
}

/// Full record context combining routing data with source metadata.
#[derive(Debug, Clone, Copy)]
pub struct RecordContext<'a> {
    routing: RoutingContext<'a>,
    publish_time: Option<u64>,
    producer_name: Option<&'a str>,
    schema: Option<&'a SchemaInfo>,
}

impl<'a> RecordContext<'a> {
    /// Create a record context from routing metadata and optional Danube metadata.
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

    /// Return the routing portion of this record context.
    pub fn routing(&self) -> RoutingContext<'a> {
        self.routing
    }

    /// Return the logical topic associated with the record.
    pub fn topic(&self) -> &'a str {
        self.routing.topic()
    }

    /// Return the routing key used for partitioned publishing, if present.
    pub fn key(&self) -> Option<&'a str> {
        self.routing.key()
    }

    /// Return the consumed partition identifier, if known.
    pub fn partition(&self) -> Option<&'a str> {
        self.routing.partition()
    }

    /// Return the user-defined attributes carried by the record.
    pub fn attributes(&self) -> &'a HashMap<String, String> {
        self.routing.attributes()
    }

    /// Return the Danube publish timestamp in microseconds since epoch, if available.
    pub fn publish_time(&self) -> Option<u64> {
        self.publish_time
    }

    /// Return the producer name that originally published the record, if available.
    pub fn producer_name(&self) -> Option<&'a str> {
        self.producer_name
    }

    /// Return schema metadata associated with the record, if available.
    pub fn schema(&self) -> Option<&'a SchemaInfo> {
        self.schema
    }
}
