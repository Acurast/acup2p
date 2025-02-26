pub mod types;

#[cfg(feature = "libp2p")]
pub mod libp2p;

use std::fmt::{self, Debug};
use std::sync::Arc;
use std::time::Duration;
use std::usize;

use async_trait::async_trait;
use futures::future::FutureExt;
use futures::lock::Mutex;
use futures::{select, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, StreamExt};

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
                    Some(Intent::OpenOutgoingStream { protocol, node, producer, consumer }) => {
                        let handler = handler.clone();
                        let open_stream = self.node.outgoing_stream(&protocol.as_str(), node.clone());
                        let open_future = async move {
                            match open_stream.await {
                                Ok(stream) => {
                                    let stream = Arc::new(Mutex::new(stream));
                                    let write_future = write_stream(stream.clone(), producer);
                                    let read_future = read_stream(stream.clone(), consumer);

                                    if cfg!(feature = "tokio") {
                                        tokio::spawn(write_future);
                                        tokio::spawn(read_future);
                                    }
                                },
                                Err(e) => handler.on_error(e).await,
                            }
                        };
                        if cfg!(feature = "tokio") {
                            tokio::spawn(open_future);
                        }
                    }
                    None => {
                        if let Err(e) = self.node.close().await {
                            handler.on_error(e).await;
                        }
                    }
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
            let subscribe_future = async move {
                while let Some((node, stream)) = next_stream.next().await {
                    handler.create_stream(node).await;

                    let consumer = handler.consumer();
                    let producer = handler.producer();
                    let stream = Arc::new(Mutex::new(stream));
                    let read_future = read_stream(stream.clone(), consumer.clone());
                    let write_future = write_stream(stream.clone(), producer.clone());

                    handler.finalize_stream().await;

                    if cfg!(feature = "tokio") {
                        tokio::spawn(read_future);
                        tokio::spawn(write_future);
                    }
                }
            };
            if cfg!(feature = "tokio") {
                tokio::spawn(subscribe_future);
            }
        }
    }
}

async fn write_stream<T>(stream: Arc<Mutex<T>>, producer: Arc<dyn StreamProducer>)
where
    T: AsyncWrite + Send + Unpin + ?Sized,
{
    while let Some(bytes) = producer.next_bytes().await {
        if let Err(e) = stream.lock().await.write_all(&bytes).await {
            producer.on_error(e).await;
        }
        producer.on_finished(StreamWrite::Ok).await;
    }
    if let Err(e) = stream.lock().await.close().await {
        producer.on_error(e).await;
    }
    producer.on_finished(StreamWrite::EOS).await;
}

async fn read_stream<T>(stream: Arc<Mutex<T>>, consumer: Arc<dyn StreamConsumer>)
where
    T: AsyncRead + Send + Unpin + ?Sized,
{
    while let Some(buffer_size) = consumer.next_read().await {
        let mut bytes = vec![0u8; buffer_size.try_into().unwrap_or(usize::MAX)];
        match stream.lock().await.read(&mut bytes).await {
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
    if let Err(e) = __bind(&handler, incoming_stream_handlers, config).await {
        handler.on_error(e).await;
    }
}

async fn __bind(
    handler: &Arc<dyn Handler>,
    incoming_stream_handlers: Vec<Arc<dyn IncomingStreamHandler>>,
    config: Config,
) -> Result<()> {
    let mut ffi;
    #[cfg(feature = "libp2p")]
    {
        ffi = FFI::libp2p(config).await?;
    }

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
