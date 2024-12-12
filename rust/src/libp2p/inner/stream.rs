use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use libp2p::swarm::InvalidProtocol;
use libp2p::{Stream, StreamProtocol};
use libp2p_stream::{self as stream, AlreadyRegistered};
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::base;

use super::super::stream::StreamMessage;
use super::NodeInner;

pub(super) struct StreamControl {
    protocol: Arc<String>,
    incoming_streams: Arc<Mutex<stream::IncomingStreams>>,
    outgoing_streams: Vec<Stream>,
}

impl StreamControl {
    pub(super) fn new(protocol: Arc<String>, behaviour: &stream::Behaviour) -> Result<Self, Error> {
        let incoming_streams = behaviour
            .new_control()
            .accept(
                StreamProtocol::try_from_owned(protocol.deref().to_owned())
                    .map_err(|e| Error::InvalidProtocol(e))?,
            )
            .map_err(|e| Error::AlreadyRegistered(e))?;

        Ok(StreamControl {
            protocol,
            incoming_streams: Arc::new(Mutex::new(incoming_streams)),
            outgoing_streams: vec![],
        })
    }

    pub(super) fn listen_incoming(
        &mut self,
        buffer_size: usize,
        tx: Arc<Mutex<Sender<StreamMessage>>>,
    ) {
        let protocol = self.protocol.clone();
        let incoming_streams = self.incoming_streams.clone();

        tokio::spawn(async move {
            while let Some((peer, mut stream)) = incoming_streams.lock().await.next().await {
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

                                if read == 0 {
                                    if let Err(e) = tx.send(StreamMessage::EndOfStream).await {
                                        tracing::debug!(error=%e, "failed to send StreamMessage::EndOfStream due to an error");
                                    }
                                } else {
                                    if let Err(e) = tx
                                        .send(StreamMessage::Frame(base::types::StreamMessage {
                                            protocol: protocol.deref().clone(),
                                            sender: base::types::NodeId::Peer {
                                                peer_id: peer.to_string(),
                                            },
                                            bytes: buf.clone(),
                                        }))
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
}

impl NodeInner {
    pub(super) fn open_incoming_streams(
        &mut self,
        protocol: String,
        buffer_size: usize,
        tx: Sender<StreamMessage>,
    ) -> Result<(), Error> {
        if let Some(control) = self.streams.get_mut(&protocol) {
            control.listen_incoming(buffer_size, Arc::new(Mutex::new(tx)));
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(super) enum Error {
    InvalidProtocol(InvalidProtocol),
    AlreadyRegistered(AlreadyRegistered),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidProtocol(invalid_protocol) => invalid_protocol.fmt(f),
            Error::AlreadyRegistered(already_registered) => already_registered.fmt(f),
        }
    }
}