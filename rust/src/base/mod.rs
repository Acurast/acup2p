pub mod types;

use async_trait::async_trait;
use futures::Stream;
use std::time::Duration;
use types::StreamMessage;

use crate::types::connection::ReconnectPolicy;
use crate::types::result::Result;

use self::types::{Event, Identity, NodeId, OutboundProtocolMessage};

#[async_trait]
pub trait Node: Stream<Item = Event> {
    async fn new(config: Config<'_>) -> Result<Self>
    where
        Self: Sized;

    async fn connect(&mut self, nodes: &[NodeId]) -> Result<()>;
    async fn disconnect(&mut self, nodes: &[NodeId]) -> Result<()>;

    async fn send_message(
        &mut self,
        message: OutboundProtocolMessage,
        nodes: &[NodeId],
    ) -> Result<()>;

    async fn stream_next(&mut self, protocol: &str) -> Option<StreamMessage>;

    async fn close(&mut self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Config<'a> {
    pub identity: Identity,

    pub msg_protocols: Vec<&'a str>,
    pub stream_protocols: Vec<(&'a str, StreamConfig)>,

    pub relay_addrs: Vec<&'a str>,

    pub reconn_policy: ReconnectPolicy,
    pub idle_conn_timeout: Duration,
}

impl Default for Config<'_> {
    fn default() -> Self {
        Self {
            identity: Identity::Random,
            msg_protocols: vec![],
            stream_protocols: vec![],
            relay_addrs: vec![],
            reconn_policy: ReconnectPolicy::Always,
            idle_conn_timeout: Duration::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub messages_buffer_size: usize,
    pub read_buffer_size: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            messages_buffer_size: 64,
            read_buffer_size: 1024,
        }
    }
}
