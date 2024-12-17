mod behaviour;
mod identity;
pub mod inner;
mod message;
mod node;
mod relay;

use async_trait::async_trait;
use futures::Stream;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use crate::base;
use crate::base::types::{Event, OutboundMessage};
use crate::types::Result;

use self::inner::NodeInner;
use self::node::NodeId;

const DEFAULT_CHANNEL_BUFFER: usize = 255;

pub struct Node {
    intent_tx: Sender<Intent>,
    event_rx: Receiver<Event>,

    incoming_stream_rx: HashMap<
        Arc<String>,
        Receiver<(base::types::NodeId, Box<dyn base::stream::IncomingStream>)>,
    >,

    outgoing_stream_tx:
        HashMap<Arc<String>, Arc<Mutex<Sender<Result<Box<dyn base::stream::OutgoingStream>>>>>>,
    outgoing_stream_rx:
        HashMap<Arc<String>, Receiver<Result<Box<dyn base::stream::OutgoingStream>>>>,
}

#[async_trait]
impl base::Node for Node {
    async fn new(config: base::Config<'_>) -> Result<Self> {
        let (event_tx, event_rx) = channel(DEFAULT_CHANNEL_BUFFER);
        let (intent_tx, intent_rx) = channel(DEFAULT_CHANNEL_BUFFER);
        let ((incoming_stream_tx, incoming_stream_rx), (outgoing_stream_tx, outgoing_stream_rx)) = config
            .stream_protocols
            .iter()
            .map(|(p, c)| {
                let (incoming_tx, incoming_rx) = channel(c.incoming_buffer_size);
                let (outgoing_tx, outgoing_rx) = channel(c.outgoing_buffer_size);

                (Arc::new((*p).to_owned()), (incoming_tx, incoming_rx), (outgoing_tx, outgoing_rx))
            })
            .fold(
                ((HashMap::new(), HashMap::new()), (HashMap::new(), HashMap::new())),
                |((mut incoming_tx_map, mut incoming_rx_map), (mut outgoing_tx_map, mut outgoing_rx_map)), (p, (incoming_tx, incoming_rx), (outgoing_tx, outgoing_rx))| {
                    incoming_tx_map.insert(p.clone(), Arc::new(Mutex::new(incoming_tx)));
                    incoming_rx_map.insert(p.clone(), incoming_rx);

                    outgoing_tx_map.insert(p.clone(), Arc::new(Mutex::new(outgoing_tx)));
                    outgoing_rx_map.insert(p.clone(), outgoing_rx);

                    ((incoming_tx_map, incoming_rx_map), (outgoing_tx_map, outgoing_rx_map))
                },
            );

        let mut inner = NodeInner::new(event_tx, intent_rx, config).await?;
        tokio::spawn(async move {
            inner.start(&incoming_stream_tx).await;
        });

        Ok(Node {
            intent_tx,
            event_rx,
            incoming_stream_rx,
            outgoing_stream_tx,
            outgoing_stream_rx,
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
        message: OutboundMessage,
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

    async fn next_incoming_stream(
        &mut self,
        protocol: &str,
    ) -> Option<(base::types::NodeId, Box<dyn base::stream::IncomingStream>)> {
        let rx = match self
            .incoming_stream_rx
            .get_mut(&Arc::new(protocol.to_owned()))
        {
            Some(rx) => rx,
            None => return None,
        };

        rx.recv().await
    }

    async fn open_outgoing_stream(
        &mut self,
        protocol: &str,
        node: base::types::NodeId,
    ) -> Result<Box<dyn base::stream::OutgoingStream>> {
        let protocol = Arc::new(protocol.to_owned());
        let tx = self
            .outgoing_stream_tx
            .get(&protocol)
            .ok_or(Error::UnknownProtocol((*protocol).clone()))?;

        let rx = self
            .outgoing_stream_rx
            .get_mut(&protocol)
            .ok_or(Error::UnknownProtocol((*protocol).clone()))?;

        self.intent_tx
            .send(Intent::OpenStream {
                peer: (&node).try_into()?,
                protocol: (*protocol).clone(),
                tx: tx.clone(),
            })
            .await?;

        let result = rx.recv().await.ok_or(Error::NodeClosed)?;

        result
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
        message: OutboundMessage,
    },
    OpenStream {
        peer: NodeId,
        protocol: String,
        tx: Arc<Mutex<Sender<Result<Box<dyn base::stream::OutgoingStream>>>>>,
    },
    Dial(NodeId),
    Disconnect(NodeId),
    Close,
}

#[derive(Debug)]
pub(self) enum Error {
    UnknownProtocol(String),
    NodeClosed,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownProtocol(protocol) => write!(f, "Unknown protocol {protocol}"),
            Error::NodeClosed => write!(f, "Node is closed"),
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
