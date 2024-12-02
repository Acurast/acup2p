use async_trait::async_trait;
use futures::Stream;
use std::time::Duration;

use crate::connection::ReconnectPolicy;
use crate::context::Context;
use crate::types::event::Event;
use crate::types::identity::Identity;
use crate::types::message::OutboundProtocolMessage;
use crate::types::node::Id;
use crate::types::result::Result;

#[async_trait]
pub trait Node: Stream<Item = Event<Self::Context>> {
    type Context: Context;

    async fn new(config: Config<'_>) -> Result<Self>
    where
        Self: Sized;

    async fn connect(&mut self, nodes: &[Id]) -> Result<()>;
    async fn disconnect(&mut self, nodes: &[Id]) -> Result<()>;

    async fn send_message(
        &mut self,
        message: OutboundProtocolMessage<Self::Context>,
        nodes: &[Id],
    ) -> Result<()>;

    async fn close(&mut self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Config<'a> {
    pub identity: Identity,
    pub msg_protocols: Vec<&'a str>,

    pub relay_addrs: Vec<&'a str>,

    pub reconn_policy: ReconnectPolicy,
    pub idle_conn_timeout: Duration,
}

impl Default for Config<'_> {
    fn default() -> Self {
        Self {
            identity: Identity::Random,
            msg_protocols: vec![],
            relay_addrs: vec![],
            reconn_policy: ReconnectPolicy::Always,
            idle_conn_timeout: Duration::ZERO,
        }
    }
}
