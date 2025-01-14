use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use futures::StreamExt;
use libp2p::swarm::InvalidProtocol;
use libp2p::{Multiaddr, PeerId, Stream, StreamProtocol};
use libp2p_stream as stream;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::base;
use crate::types::{MaybeFrom, Result};

use super::{NodeId, NodeInner};

impl base::stream::IncomingStream for Stream {}
impl base::stream::OutgoingStream for Stream {}

pub(super) struct StreamControl {
    protocol: StreamProtocol,
    control: stream::Control,
}

impl StreamControl {
    pub(super) fn new(protocol: Arc<String>, behaviour: &stream::Behaviour) -> Result<Self, Error> {
        let protocol = StreamProtocol::try_from_owned(protocol.deref().to_owned())
            .map_err(|e| Error::InvalidProtocol(e))?;
        let control = behaviour.new_control();

        Ok(StreamControl { protocol, control })
    }

    fn subscribe_incoming(
        &mut self,
        tx: Arc<Mutex<Sender<(base::types::NodeId, Box<dyn base::stream::IncomingStream>)>>>,
    ) {
        let mut incoming_streams = match self.control.accept(self.protocol.clone()) {
            Ok(incoming_streams) => incoming_streams,
            Err(stream::AlreadyRegistered) => return,
        };
        let tx = tx.clone();

        tokio::spawn(async move {
            while let Some((peer, stream)) = incoming_streams.next().await {
                if let Err(_) = tx
                    .lock()
                    .await
                    .send((
                        base::types::NodeId::Peer {
                            peer_id: peer.to_string(),
                        },
                        Box::new(stream),
                    ))
                    .await
                {
                    tracing::debug!("failed to send incoming stream, channel is closed");
                }
            }
        });
    }

    async fn open_outgoing(
        &mut self,
        node: NodeId,
        tx: Arc<Mutex<Sender<Result<Box<dyn base::stream::OutgoingStream>>>>>,
    ) -> Result<(), Error> {
        let peer_id = match node {
            NodeId::Peer(peer_id) => peer_id,
            NodeId::Addr(addr) => {
                PeerId::maybe_from(addr.clone()).ok_or(Error::InvalidAddress(addr))?
            }
        };

        match self
            .control
            .open_stream(peer_id, self.protocol.clone())
            .await
        {
            Ok(stream) => {
                if let Err(_) = tx.lock().await.send(Ok(Box::new(stream))).await {
                    tracing::debug!("failed to send outgoing stream, channel is closed");
                }
            }
            Err(e) => return Err(Error::OpenStream(e)),
        }

        Ok(())
    }
}

impl NodeInner {
    pub(super) fn subscribe_incoming_streams(
        &mut self,
        tx: &HashMap<
            Arc<String>,
            Arc<Mutex<Sender<(base::types::NodeId, Box<dyn base::stream::IncomingStream>)>>>,
        >,
    ) {
        for control in self.streams.values_mut() {
            let tx = match tx.get(&Arc::new(control.protocol.as_ref().to_owned())) {
                Some(tx) => tx,
                None => continue,
            };

            control.subscribe_incoming(tx.clone());
        }
    }

    pub(super) async fn open_outgoing_stream(
        &mut self,
        protocol: String,
        node: NodeId,
        tx: Arc<Mutex<Sender<Result<Box<dyn base::stream::OutgoingStream>>>>>,
    ) -> Result<(), Error> {
        let control = self
            .streams
            .get_mut(&Arc::new(protocol.clone()))
            .ok_or(Error::UnknownProtocol(protocol))?;

        control.open_outgoing(node, tx).await?;

        Ok(())
    }
}

#[derive(Debug)]
pub(super) enum Error {
    InvalidProtocol(InvalidProtocol),
    UnknownProtocol(String),
    InvalidAddress(Multiaddr),
    OpenStream(stream::OpenStreamError),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidProtocol(invalid_protocol) => invalid_protocol.fmt(f),
            Error::UnknownProtocol(protocol) => write!(f, "Unknown protocol {protocol}"),
            Error::InvalidAddress(multiaddr) => write!(f, "Address {multiaddr} is invalid"),
            Error::OpenStream(e) => e.fmt(f),
        }
    }
}
