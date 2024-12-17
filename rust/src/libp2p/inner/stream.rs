use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use libp2p::swarm::InvalidProtocol;
use libp2p::{PeerId, Stream, StreamProtocol};
use libp2p_stream as stream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;

use crate::base;
use crate::types::MaybeFrom;

use super::super::stream::StreamMessage;
use super::{NodeId, NodeInner};

pub(super) struct StreamControl {
    protocol: StreamProtocol,
    buffer_size: usize,
    control: stream::Control,
    streams: Arc<Mutex<HashMap<PeerId, Stream>>>,
}

impl StreamControl {
    pub(super) fn new(
        protocol: Arc<String>,
        buffer_size: usize,
        behaviour: &stream::Behaviour,
    ) -> Result<Self, Error> {
        let protocol = StreamProtocol::try_from_owned(protocol.deref().to_owned())
            .map_err(|e| Error::InvalidProtocol(e))?;
        let control = behaviour.new_control();

        Ok(StreamControl {
            protocol,
            buffer_size,
            control,
            streams: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn open_incoming(&mut self, tx: Arc<Mutex<Sender<StreamMessage>>>) {
        let mut incoming_streams = match self.control.accept(self.protocol.clone()) {
            Ok(incoming_streams) => incoming_streams,
            Err(stream::AlreadyRegistered) => return,
        };

        let protocol = Arc::new(self.protocol.as_ref().to_owned());
        let buffer_size = self.buffer_size;

        tokio::spawn(async move {
            while let Some((peer, mut stream)) = incoming_streams.next().await {
                let protocol = protocol.clone();
                let tx = tx.clone();

                tokio::spawn(async move {
                    let mut buf = vec![0u8; buffer_size];
                    loop {
                        match stream.read(&mut buf).await {
                            Ok(read) => {
                                let tx = tx.lock().await;
                                if tx.is_closed() {
                                    if let Err(e) = stream.close().await {
                                        tracing::debug!(error=%e, "failed to close incoming stream due to an error");
                                    }
                                    break;
                                }

                                let node = NodeId::Peer(peer);
                                if read == 0 {
                                    if let Err(e) = tx.send(StreamMessage::EOS(node)).await {
                                        tracing::debug!(error=%e, "failed to send StreamMessage::EOS due to an error");
                                    }
                                } else {
                                    if let Err(e) = tx
                                        .send(StreamMessage::Frame((
                                            node,
                                            base::types::StreamMessage {
                                                protocol: protocol.deref().clone(),
                                                bytes: buf.clone(),
                                            },
                                        )))
                                        .await
                                    {
                                        tracing::debug!(error=%e, "failed to send StreamMessage::Frame due to an error");
                                    }
                                }
                            }
                            Err(_) => todo!(),
                        }
                    }
                });
            }
        });
    }

    fn subscribe_outgoing(&mut self, rx: Arc<Mutex<Receiver<StreamMessage>>>) {
        let protocol = self.protocol.clone();
        let mut control = self.control.clone();
        let streams = self.streams.clone();

        tokio::spawn(async move {
            while let Some(message) = rx.lock().await.recv().await {
                let node = match &message {
                    StreamMessage::Frame((node, _)) => node,
                    StreamMessage::EOS(node) => node,
                };

                let peer_id = match node {
                    NodeId::Peer(peer_id) => peer_id.clone(),
                    NodeId::Addr(addr) => {
                        match PeerId::maybe_from(addr.clone()) {
                            Some(peer_id) => peer_id,
                            None => {
                                tracing::debug!(%addr, "can't write to stream, invalid address");
                                continue;
                            },
                        }
                    }
                };

                let mut streams = streams.lock().await;
                let stream = match streams.entry(peer_id.clone()) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => {
                        let stream = match control.open_stream(peer_id, protocol.clone()).await {
                            Ok(stream) => stream,
                            Err(e @ stream::OpenStreamError::UnsupportedProtocol(_)) => {
                                // TODO
                                continue;
                            },
                            Err(_) => {
                                // TODO: retry?
                                continue;
                            }
                        };
                        
                        entry.insert(stream)
                    },
                };

                match message {
                    StreamMessage::Frame((_, message)) => {
                        if let Err(e) = stream.write_all(&message.bytes).await {
                            // TODO
                        }
                    },
                    StreamMessage::EOS(_) => {
                        if let Err(e) = stream.close().await {
                            // TODO
                        }
                    },
                };
            }
        });
    }
}

impl NodeInner {
    pub(super) fn open_incoming_streams(
        &mut self,
        tx: &HashMap<Arc<String>, Arc<Mutex<Sender<StreamMessage>>>>,
    ) {
        for control in self.streams.values_mut() {
            let tx = match tx.get(&Arc::new(control.protocol.as_ref().to_owned())) {
                Some(tx) => tx,
                None => continue,
            };
            control.open_incoming(tx.clone());
        }
    }

    pub(super) fn subscribe_outgoing_streams(
        &mut self,
        rx: &HashMap<Arc<String>, Arc<Mutex<Receiver<StreamMessage>>>>,
    ) {
        for control in self.streams.values_mut() {
            let rx = match rx.get(&Arc::new(control.protocol.as_ref().to_owned())) {
                Some(rx) => rx,
                None => continue,
            };
            control.subscribe_outgoing(rx.clone());
        }
    }
}

#[derive(Debug)]
pub(super) enum Error {
    InvalidProtocol(InvalidProtocol),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidProtocol(invalid_protocol) => invalid_protocol.fmt(f),
        }
    }
}
