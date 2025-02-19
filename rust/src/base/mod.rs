pub mod stream;
pub mod types;

use async_trait::async_trait;
use futures::Stream;
use std::future::Future;
use std::time::Duration;
use stream::OutgoingStream;

use crate::types::connection::ReconnectPolicy;
use crate::types::result::Result;

use self::stream::IncomingStream;
use self::types::{Event, Identity, NodeId, OutboundProtocolMessage};

#[async_trait]
pub trait Node: Stream<Item = Event> {
    type Log;

    async fn new(config: Config<'_, Self::Log>) -> Result<Self>
    where
        Self: Sized;

    async fn connect(&mut self, nodes: &[NodeId]) -> Result<()>;
    async fn disconnect(&mut self, nodes: &[NodeId]) -> Result<()>;

    async fn send_message(
        &mut self,
        message: OutboundProtocolMessage,
        nodes: &[NodeId],
    ) -> Result<()>;

    fn incoming_streams(
        &mut self,
        protocol: &str,
    ) -> impl Stream<Item = (NodeId, Box<dyn IncomingStream>)> + Send + Unpin + 'static;
    fn outgoing_stream(
        &mut self,
        protocol: &str,
        node: NodeId,
    ) -> impl Future<Output = Result<Box<dyn OutgoingStream>>> + Send + 'static;

    async fn close(&mut self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Config<'a, L> {
    pub identity: Identity,

    pub msg_protocols: Vec<&'a str>,
    pub stream_protocols: Vec<&'a str>,

    pub relay_addrs: Vec<&'a str>,

    pub reconn_policy: ReconnectPolicy,
    pub idle_conn_timeout: Duration,

    pub log: L,
}

impl<L> Default for Config<'_, L>
where
    L: Default,
{
    fn default() -> Self {
        Self {
            identity: Identity::Random,
            msg_protocols: vec![],
            stream_protocols: vec![],
            relay_addrs: vec![],
            reconn_policy: ReconnectPolicy::Always,
            idle_conn_timeout: Duration::ZERO,
            log: Default::default(),
        }
    }
}
