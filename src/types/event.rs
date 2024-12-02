use std::fmt;

use super::context::Context;
use super::message::{
    InboundProtocolRequest, InboundProtocolResponse, OutboundProtocolRequest,
    OutboundProtocolResponse,
};
use super::node::Id;

#[derive(Debug, Clone)]
pub enum Event<C>
where
    C: Context,
{
    ListeningOn(String),

    Connected(Id),
    Disconnected(Id),

    InboundRequest(Id, InboundProtocolRequest<C>),
    InboundResponse(Id, InboundProtocolResponse<C>),

    OutboundRequest(Id, OutboundProtocolRequest),
    OutboundResponse(Id, OutboundProtocolResponse<C>),

    Error(String),
}

impl<C> fmt::Display for Event<C>
where
    C: Context,
{
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
