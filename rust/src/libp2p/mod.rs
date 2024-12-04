mod behaviour;
mod identity;
pub mod inner;
mod message;
mod node;
mod relay;

use async_trait::async_trait;
use futures::Stream;
use tokio::spawn;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use crate::base;
use crate::base::types::{Event, OutboundProtocolMessage};
use crate::types::Result;

use self::inner::NodeInner;
use self::node::NodeId;

const DEFAULT_CHANNEL_BUFFER: u8 = 255;

pub struct Node {
    intent_tx: Sender<Intent>,
    event_rx: Receiver<Event>,
}

#[derive(Debug, Clone)]
pub(self) enum Intent {
    DirectMessage {
        peer: NodeId,
        message: OutboundProtocolMessage,
    },
    Dial(NodeId),
    Disconnect(NodeId),
    Close,
}

impl Stream for Node {
    type Item = Event;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.as_mut().event_rx.poll_recv(cx)
    }
}

#[async_trait]
impl base::Node for Node {
    async fn new(config: base::Config<'_>) -> Result<Self> {
        let (event_tx, event_rx) = channel(usize::from(DEFAULT_CHANNEL_BUFFER));
        let (intent_tx, intent_rx) = channel(usize::from(DEFAULT_CHANNEL_BUFFER));

        let mut inner = NodeInner::new(event_tx, intent_rx, config).await?;
        spawn(async move {
            inner.start().await;
        });

        Ok(Node {
            intent_tx,
            event_rx,
        })
    }

    async fn connect(&mut self, nodes: &[base::types::NodeId]) -> Result<()> {
        for node in nodes {
            self.intent_tx.send(Intent::Dial(node.try_into()?)).await?;
        }

        Ok(())
    }

    async fn disconnect(&mut self, nodes: &[base::types::NodeId]) -> Result<()> {
        for node in nodes {
            self.intent_tx
                .send(Intent::Disconnect(node.try_into()?))
                .await?;
        }

        Ok(())
    }

    async fn send_message(
        &mut self,
        message: OutboundProtocolMessage,
        nodes: &[base::types::NodeId],
    ) -> Result<()> {
        for node in nodes {
            self.intent_tx
                .send(Intent::DirectMessage {
                    peer: node.try_into()?,
                    message: message.clone(),
                })
                .await?;
        }

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.intent_tx.send(Intent::Close).await?;
        self.event_rx.close();

        Ok(())
    }
}

impl Node {
    pub fn enable_log(config: LogConfig) {
        if let Err(e) = Self::init_tracing_subscriber(&config) {
            match config.level_filter {
                LevelFilter::OFF => {}
                _ => println!("Failed to init tracing: {e}"),
            }
        }
    }

    fn init_tracing_subscriber(config: &LogConfig) -> Result<()> {
        tracing_subscriber::fmt()
            .with_ansi(config.with_ansi)
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive(config.level_filter.into())
                    .from_env()?,
            )
            .init();

        Ok(())
    }
}

pub struct LogConfig {
    pub with_ansi: bool,
    pub level_filter: LevelFilter,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            with_ansi: true,
            level_filter: LevelFilter::INFO,
        }
    }
}
