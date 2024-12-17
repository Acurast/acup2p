use std::fmt;

use libp2p::{Multiaddr, PeerId};

use crate::base::types::OutboundMessage;
use crate::types::{MaybeFrom, Result};

use super::super::message;
use super::super::node::NodeId;
use super::NodeInner;

impl NodeInner {
    pub(super) async fn send_direct_message(
        &mut self,
        node: NodeId,
        message: OutboundMessage,
    ) -> Result<(), Error> {
        match message {
            OutboundMessage::Request(message) => {
                self.send_request(&node, &message.protocol, &message.bytes)?;
                self.notify_outbound_request(&node, message).await;
            }
            OutboundMessage::Response(message) => {
                self.send_response(&node, &message.protocol, &message.bytes, &message.id)?;
                self.notify_outbound_response(&node, message).await;
            }
        }

        Ok(())
    }

    fn send_request(
        &mut self,
        node: &NodeId,
        protocol: &String,
        bytes: &Vec<u8>,
    ) -> Result<(), Error> {
        let behaviour = self.get_message_behaviour(&protocol)?;
        let peer_id = match node {
            NodeId::Peer(peer_id) => peer_id,
            NodeId::Addr(addr) => {
                &PeerId::maybe_from(addr.clone()).ok_or(Error::InvalidAddress(addr.clone()))?
            }
        };

        behaviour.send_request(peer_id, bytes.clone());

        Ok(())
    }

    fn send_response(
        &mut self,
        node: &NodeId,
        protocol: &String,
        bytes: &Vec<u8>,
        request_id: &String,
    ) -> Result<(), Error> {
        let response_key = (node.clone(), protocol.clone(), request_id.clone());
        let response_channel = self
            .response_channels
            .remove(&response_key)
            .ok_or(Error::ResponseChannelNotFound(response_key.clone()))?;

        let behaviour = self.get_message_behaviour(&protocol)?;
        behaviour
            .send_response(response_channel, bytes.clone())
            .map_err(|_| Error::ResponseChannelClosed(response_key))?;

        Ok(())
    }

    fn get_message_behaviour(
        &mut self,
        protocol: &String,
    ) -> Result<&mut message::Behaviour, Error> {
        self.swarm
            .behaviour_mut()
            .messages
            .get_mut(protocol)
            .ok_or(Error::MessageProtocolNotFound(protocol.clone()))
    }
}

#[derive(Debug)]
pub(super) enum Error {
    MessageProtocolNotFound(String),

    InvalidAddress(Multiaddr),

    ResponseChannelNotFound((NodeId, String, String)),
    ResponseChannelClosed((NodeId, String, String)),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MessageProtocolNotFound(protocol) => {
                write!(f, "Message protocol {protocol} was not found")
            }
            Error::InvalidAddress(multiaddr) => write!(f, "Address {multiaddr} is invalid"),
            Error::ResponseChannelNotFound((node, protocol, request_id)) => write!(
                f,
                "Response channel for ({node}, {protocol}, {request_id}) was not found"
            ),
            Error::ResponseChannelClosed((node, protocol, request_id)) => write!(
                f,
                "Response channel for ({node}, {protocol}, {request_id}) is closed"
            ),
        }
    }
}
