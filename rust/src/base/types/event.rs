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
    ListeningOn(String),

    Connected(NodeId),
    Disconnected(NodeId),

    InboundRequest(NodeId, InboundProtocolRequest),
    InboundResponse(NodeId, InboundProtocolResponse),

    OutboundRequest(NodeId, OutboundProtocolRequest),
    OutboundResponse(NodeId, OutboundProtocolResponse),

    Error(String),
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::ListeningOn(addr) => write!(f, "Listening on address {addr}"),
            Event::Connected(id) => write!(f, "Node {id} connected"),
            Event::Disconnected(id) => write!(f, "Node {id} disconnected"),
            Event::InboundRequest(id, message) => {
                write!(f, "Received a request from {id}: {message}")
            }
            Event::InboundResponse(id, message) => {
                write!(f, "Received a response from {id}: {message}")
            }
            Event::OutboundRequest(id, message) => {
                write!(f, "Sent a request to {id}: {message}")
            }
            Event::OutboundResponse(id, message) => {
                write!(f, "Sent a response to {id}: {message}")
            }
            Event::Error(e) => write!(f, "Error: {e}"),
        }
    }
}
