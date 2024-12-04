pub mod types;

#[cfg(feature = "libp2p")]
pub mod libp2p;

use std::error::Error;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::future::FutureExt;
use futures::{select, StreamExt};

use crate::base::types::OutboundProtocolMessage;
use crate::base::{self, Node};
use crate::types::Result;

use self::types::{Event, Identity, NodeId, ReconnectPolicy};

pub struct FFI<T>
where
    T: Node + Unpin,
{
    node: T,
}

impl<T> FFI<T>
where
    T: Node + Unpin,
{
    async fn run_loop(&mut self, handler: &Arc<dyn Handler>) {
        loop {
            select! {
                event = self.node.next().fuse() => match event {
                    Some(event) => handler.on_event(event).await,
                    None => break,
                },
                intent = handler.next_intent().fuse() => match intent {
                    Some(Intent::Connect { nodes }) => {
                        if let Err(e) = self.node.connect(&nodes).await {
                            handler.on_error(e).await;
                        }
                    }
                    Some(Intent::Disconnect { nodes }) => {
                        if let Err(e) = self.node.disconnect(&nodes).await {
                            handler.on_error(e).await;
                        }
                    }
                    Some(Intent::SendMessage { message, nodes }) => {
                        if let Err(e) = self.node.send_message(message, &nodes).await {
                            handler.on_error(e).await;
                        }
                    }
                    Some(Intent::Close) | None => {
                        if let Err(e) = self.node.close().await {
                            handler.on_error(e).await;
                        }
                    }
                }
            }
        }
    }
}

#[uniffi::export]
pub fn default_config() -> Config {
    Config::default()
}

#[cfg_attr(feature = "tokio", uniffi::export(async_runtime = "tokio"))]
#[cfg_attr(not(feature = "tokio"), uniffi::export)]
pub async fn bind(handler: Arc<dyn Handler>, config: Config) {
    if let Err(e) = __bind(&handler, config).await {
        handler.on_error(e).await;
    }
}

async fn __bind(handler: &Arc<dyn Handler>, config: Config) -> Result<()> {
    let mut ffi;
    #[cfg(feature = "libp2p")]
    {
        ffi = FFI::libp2p(config).await?;
    }

    ffi.run_loop(handler).await;

    Ok(())
}

#[uniffi::export(with_foreign)]
#[async_trait]
pub trait Handler: Send + Sync + Debug {
    async fn on_event(&self, event: Event);
    async fn next_intent(&self) -> Option<Intent>;
}

impl dyn Handler {
    async fn on_error(&self, error: Box<dyn Error + Send + Sync>) {
        self.on_event(Event::Error {
            cause: error.to_string(),
        })
        .await;
    }
}

#[derive(uniffi::Enum)]
pub enum Intent {
    Connect {
        nodes: Vec<NodeId>,
    },
    Disconnect {
        nodes: Vec<NodeId>,
    },
    SendMessage {
        message: OutboundProtocolMessage,
        nodes: Vec<NodeId>,
    },
    Close,
}

#[derive(uniffi::Record)]
pub struct Config {
    pub identity: Identity,
    pub message_protocols: Vec<String>,
    pub relay_addresses: Vec<String>,
    pub reconnect_policy: ReconnectPolicy,
    pub idle_connection_timeout: Duration,
    pub log_level: LogLevel,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            identity: Identity::Random,
            message_protocols: vec![],
            relay_addresses: vec![],
            reconnect_policy: ReconnectPolicy::Always,
            idle_connection_timeout: Duration::from_secs(15),
            log_level: LogLevel::Info,
        }
    }
}

impl<'a> From<&'a Config> for base::Config<'a> {
    fn from(value: &'a Config) -> Self {
        base::Config {
            identity: value.identity.clone().into(),
            msg_protocols: value.message_protocols.iter().map(|s| s.as_str()).collect(),
            relay_addrs: value.relay_addresses.iter().map(|s| s.as_str()).collect(),
            reconn_policy: value.reconnect_policy,
            idle_conn_timeout: value.idle_connection_timeout,
        }
    }
}

#[derive(uniffi::Enum, Debug, Clone, Copy)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
