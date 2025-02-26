use std::fmt;

use super::message::{
    InboundProtocolRequest, InboundProtocolResponse, OutboundProtocolRequest,
    OutboundProtocolResponse,
};

use super::node::NodeId;

#[cfg_attr(
    any(target_os = "android", target_os = "ios"),
    derive(uniffi::Enum, Debug, Clone)
)]
#[cfg_attr(
    not(any(target_os = "android", target_os = "ios")),
    derive(Debug, Clone)
)]
pub enum Event {
    ListeningOn {
        address: String,
    },
    Ready,

    Connected {
        node: NodeId,
    },
    Disconnected {
        node: NodeId,
    },
    ConnectionUpgraded {
        node: NodeId,
    },
    ConnectionError {
        node: NodeId,
        cause: String,
    },

    InboundRequest {
        sender: NodeId,
        request: InboundProtocolRequest,
    },
    InboundResponse {
        sender: NodeId,
        response: InboundProtocolResponse,
    },

    OutboundRequest {
        receiver: NodeId,
        request: OutboundProtocolRequest,
    },
    OutboundResponse {
        receiver: NodeId,
        response: OutboundProtocolResponse,
    },

    Error {
        cause: String,
    },
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::ListeningOn { address } => write!(f, "Listening on address {address}"),
            Event::Ready => write!(f, "Ready"),
            Event::Connected { node } => write!(f, "Node {node} connected"),
            Event::Disconnected { node } => write!(f, "Node {node} disconnected"),
            Event::ConnectionUpgraded { node } => write!(f, "Connection to node {node} upgraded"),
            Event::ConnectionError { node, cause } => {
                write!(f, "Failed to connect to {node}: {cause}")
            }
            Event::InboundRequest { sender, request } => {
                write!(f, "Received a request from {sender}: {request}")
            }
            Event::InboundResponse { sender, response } => {
                write!(f, "Received a response from {sender}: {response}")
            }
            Event::OutboundRequest { receiver, request } => {
                write!(f, "Sent a request to {receiver}: {request}")
            }
            Event::OutboundResponse { receiver, response } => {
                write!(f, "Sent a response to {receiver}: {response}")
            }
            Event::Error { cause } => write!(f, "Error: {cause}"),
        }
    }
}
