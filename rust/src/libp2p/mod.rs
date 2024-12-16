mod behaviour;
mod identity;
pub mod inner;
mod message;
mod node;
mod relay;
mod stream;

use async_trait::async_trait;
use futures::Stream;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use crate::base;
use crate::base::types::{Event, OutboundProtocolMessage};
use crate::types::Result;

use self::inner::NodeInner;
use self::node::NodeId;
use self::stream::StreamMessage;

const DEFAULT_CHANNEL_BUFFER: usize = 255;

pub struct Node {
    intent_tx: Sender<Intent>,
    event_rx: Receiver<Event>,

    stream_rx: HashMap<Arc<String>, Receiver<StreamMessage>>,
}

#[async_trait]
impl base::Node for Node {
    async fn new(config: base::Config<'_>) -> Result<Self> {
        let (event_tx, event_rx) = channel(DEFAULT_CHANNEL_BUFFER);
        let (intent_tx, intent_rx) = channel(DEFAULT_CHANNEL_BUFFER);
        let (stream_tx, stream_rx) = config
            .stream_protocols
            .iter()
            .map(|(p, c)| {
                let (tx, rx) = channel(c.messages_buffer_size);

                (Arc::new((*p).to_owned()), tx, rx)
            })
            .fold(
                (HashMap::new(), HashMap::new()),
                |(mut tx_map, mut rx_map), (p, tx, rx)| {
                    tx_map.insert(p.clone(), Arc::new(Mutex::new(tx)));
                    rx_map.insert(p.clone(), rx);

                    (tx_map, rx_map)
                },
            );

        let mut inner = NodeInner::new(event_tx, intent_rx, stream_tx, config).await?;
        spawn(async move {
            inner.start().await;
        });

        Ok(Node {
            intent_tx,
            event_rx,
            stream_rx,
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

    async fn stream_next(&mut self, protocol: &str) -> Option<base::types::StreamMessage> {
        let rx = match self.stream_rx.get_mut(&Arc::new(protocol.to_owned())) {
            Some(rx) => rx,
            None => return None,
        };

        match rx.recv().await {
            Some(StreamMessage::Frame(stream_message)) => Some(stream_message),
            _ => None,
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.intent_tx.send(Intent::Close).await?;
        self.event_rx.close();

        Ok(())
    }
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

#[derive(Debug)]
pub(self) enum Error {
    UnknownProtocol(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownProtocol(protocol) => write!(f, "Unknown protocol {protocol}"),
        }
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
