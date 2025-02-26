pub mod types;

#[cfg(feature = "libp2p")]
pub mod libp2p;

use std::fmt::{self, Debug};
use std::sync::Arc;
use std::time::Duration;
use std::usize;

use async_trait::async_trait;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, StreamExt};

use crate::base::types::OutboundProtocolMessage;
use crate::base::{self, Node};
use crate::types::Result;

use self::types::{Event, Identity, NodeId, PublicKey, ReconnectPolicy};

macro_rules! ffi {
    ($cfg:expr) => {{
        #[cfg(feature = "libp2p")]
        {
            FFI::libp2p($cfg)
        }
    }};
}

macro_rules! spawn {
    ($fut:expr) => {{
        #[cfg(feature = "tokio")]
        {
            tokio::spawn($fut)
        }
    }};
}

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
    #[cfg(feature = "tokio")]
    async fn run_loop(&mut self, handler: Arc<dyn Handler>) {
        use tokio::{select, spawn, sync::mpsc::channel};

        let (intents_tx, mut intents_rx) = channel(64);
        {
            let handler = handler.clone();
            spawn(async move {
                let mut closed = false;
                while !closed {
                    let intent = handler.next_intent().await;
                    closed = intent.is_none();

                    if let Err(e) = intents_tx.send(intent).await {
                        handler.on_error(e).await;
                    }
                }
            });
        }

        loop {
            select! {
                event = self.node.next() => match event {
                    Some(event) => self.on_event(&handler, event).await,
                    None => break,
                },
                intent = intents_rx.recv() => match intent {
                    Some(intent) => self.on_intent(&handler, intent).await,
                    None => intents_rx.close(),
                }
            }
        }
    }

    async fn on_event(&mut self, handler: &Arc<dyn Handler>, event: Event){
        handler.on_event(event).await;
    }

    async fn on_intent(&mut self, handler: &Arc<dyn Handler>, intent: Option<Intent>) {
        match intent {
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
            Some(Intent::OpenOutgoingStream { protocol, node, producer, consumer }) => {
                let handler = handler.clone();
                let open_stream = self.node.outgoing_stream(&protocol.as_str(), node.clone());
                spawn!(async move {
                    match open_stream.await {
                        Ok(stream) => {
                            let (read, write) = stream.split();
                            spawn!(write_stream(write, producer));
                            spawn!(read_stream(read, consumer));
                        },
                        Err(e) => handler.on_error(e).await,
                    }
                });
            },
            None => {
                if let Err(e) = self.node.close().await {
                    handler.on_error(e).await;
                }
            }
        }
    }

    fn read_incoming_streams(
        &mut self,
        incoming_stream_handlers: Vec<Arc<dyn IncomingStreamHandler>>,
    ) {
        for handler in incoming_stream_handlers.into_iter() {
            let protocol = handler.protocol();

            let mut next_stream = self.node.incoming_streams(&protocol.as_str());
            spawn!(async move {
                while let Some((node, stream)) = next_stream.next().await {
                    handler.create_stream(node).await;

                    let consumer = handler.consumer();
                    let producer = handler.producer();                    
                    handler.finalize_stream().await;
                    
                    let (read, write) = stream.split();
                    spawn!(read_stream(read, consumer.clone()));
                    spawn!(write_stream(write, producer.clone()));
                }
            });
        }
    }
}

async fn write_stream<T>(mut stream: T, producer: Arc<dyn StreamProducer>)
where
    T: AsyncWrite + Send + Unpin + Sized,
{
    while let Some(bytes) = producer.next_bytes().await {
        if let Err(e) = stream.write_all(&bytes).await {
            producer.on_error(e).await;
        }
        producer.on_finished(StreamWrite::Ok).await;
    }
    if let Err(e) = stream.close().await {
        producer.on_error(e).await;
    }
    producer.on_finished(StreamWrite::EOS).await;
}

async fn read_stream<T>(mut stream: T, consumer: Arc<dyn StreamConsumer>)
where
    T: AsyncRead + Send + Unpin + Sized,
{
    while let Some(buffer_size) = consumer.next_read().await {
        let mut bytes = vec![0u8; buffer_size.try_into().unwrap_or(usize::MAX)];
        match stream.read(&mut bytes).await {
            Ok(read) => {
                if read == 0 {
                    consumer.on_bytes(StreamRead::EOS).await;
                } else {
                    bytes.truncate(read);
                    consumer.on_bytes(StreamRead::Ok(bytes)).await;
                }
            }
            Err(e) => consumer.on_bytes(StreamRead::Err(e.to_string())).await,
        }
    }
    consumer.on_bytes(StreamRead::EOS).await;
}

#[uniffi::export]
pub fn default_config() -> Config {
    Config::default()
}

#[cfg_attr(feature = "tokio", uniffi::export(async_runtime = "tokio"))]
#[cfg_attr(not(feature = "tokio"), uniffi::export)]
pub async fn bind(
    handler: Arc<dyn Handler>,
    incoming_stream_handlers: Vec<Arc<dyn IncomingStreamHandler>>,
    config: Config,
) {
    if let Err(e) = __bind(handler.clone(), incoming_stream_handlers, config).await {
        handler.on_error(e).await;
    }
}

async fn __bind(
    handler: Arc<dyn Handler>,
    incoming_stream_handlers: Vec<Arc<dyn IncomingStreamHandler>>,
    config: Config,
) -> Result<()> {
    let mut ffi = ffi!(config).await?;
    ffi.read_incoming_streams(incoming_stream_handlers);
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
    async fn on_error<T>(&self, error: T)
    where
        T: fmt::Display,
    {
        self.on_event(Event::Error {
            cause: error.to_string(),
        })
        .await;
    }
}

#[uniffi::export(with_foreign)]
#[async_trait]
pub trait IncomingStreamHandler: Send + Sync + Debug {
    fn protocol(&self) -> String;
    fn consumer(&self) -> Arc<dyn StreamConsumer>;
    fn producer(&self) -> Arc<dyn StreamProducer>;

    async fn create_stream(&self, node: NodeId);
    async fn finalize_stream(&self);
}

#[derive(uniffi::Enum)]
pub enum StreamRead {
    Ok(Vec<u8>),
    EOS,
    Err(String),
}

#[uniffi::export(with_foreign)]
#[async_trait]
pub trait StreamConsumer: Send + Sync + Debug {
    async fn next_read(&self) -> Option<u32>;
    async fn on_bytes(&self, read: StreamRead);
}

#[derive(uniffi::Enum)]
pub enum StreamWrite {
    Ok,
    EOS,
    Err(String),
}

#[uniffi::export(with_foreign)]
#[async_trait]
pub trait StreamProducer: Send + Sync + Debug {
    async fn next_bytes(&self) -> Option<Vec<u8>>;
    async fn on_finished(&self, write: StreamWrite);
}

impl dyn StreamProducer {
    async fn on_error<T>(&self, error: T)
    where
        T: fmt::Display,
    {
        self.on_finished(StreamWrite::Err(error.to_string())).await;
    }
}

#[derive(uniffi::Enum, Debug, Clone)]
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
    OpenOutgoingStream {
        protocol: String,
        node: NodeId,
        producer: Arc<dyn StreamProducer>,
        consumer: Arc<dyn StreamConsumer>,
    },
}

#[derive(uniffi::Record)]
pub struct Config {
    pub identity: Identity,
    pub message_protocols: Vec<String>,
    pub stream_protocols: Vec<String>,
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
            stream_protocols: vec![],
            relay_addresses: vec![],
            reconnect_policy: ReconnectPolicy::Always,
            idle_connection_timeout: Duration::from_secs(15),
            log_level: LogLevel::Info,
        }
    }
}

impl Config {
    pub(crate) fn into_base<'a, L>(&'a self, log: L) -> base::Config<'a, L> {
        base::Config {
            identity: self.identity.clone().into(),
            msg_protocols: self.message_protocols.iter().map(|s| s.as_str()).collect(),
            stream_protocols: self.stream_protocols.iter().map(|s| s.as_str()).collect(),
            relay_addrs: self.relay_addresses.iter().map(|s| s.as_str()).collect(),
            reconn_policy: self.reconnect_policy,
            idle_conn_timeout: self.idle_connection_timeout,
            log,
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

#[derive(uniffi::Enum, Debug)]
enum Error {
    DecodingError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DecodingError(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for Error {}

#[uniffi::export]
fn node_id_from_public_key(pk: PublicKey) -> Result<NodeId, Error> {
    let pk = base::types::PublicKey::from(pk);
    let node_id = pk.try_into().map_err(|err| Error::DecodingError(err))?;

    Ok(node_id)
}
