use std::fmt;

use super::context::{Context, Empty};

pub type InboundProtocolRequest<C: Context> = ProtocolMessage<C::InboundRequest>;
pub type InboundProtocolResponse<C: Context> = ProtocolMessage<C::InboundResponse>;

pub type OutboundProtocolRequest = ProtocolMessage<Empty>;
pub type OutboundProtocolResponse<C: Context> = ProtocolMessage<C::InboundRequest>;

#[derive(Debug, Clone)]
pub enum OutboundProtocolMessage<C>
where
    C: Context,
{
    Request(OutboundProtocolRequest),
    Response(OutboundProtocolResponse<C>),
}

impl<C> OutboundProtocolMessage<C>
where
    C: Context,
{
    pub fn new_request(protocol: String, bytes: Vec<u8>) -> Self {
        Self::Request(ProtocolMessage {
            protocol,
            bytes,
            context: Empty(),
        })
    }

    pub fn new_response(request: ProtocolMessage<C::InboundRequest>, bytes: Vec<u8>) -> Self {
        Self::Response(ProtocolMessage {
            protocol: request.protocol,
            bytes,
            context: request.context,
        })
    }
}

impl<C> fmt::Display for OutboundProtocolMessage<C>
where
    C: Context,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutboundProtocolMessage::Request(message) => write!(f, "Request({message})"),
            OutboundProtocolMessage::Response(message) => write!(f, "Response({message})"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProtocolMessage<C>
where
    C: Clone,
{
    pub protocol: String,
    pub bytes: Vec<u8>,
    pub(crate) context: C,
}

impl<C> fmt::Display for ProtocolMessage<C>
where
    C: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProtocolMessage({}: {:02x?})", self.protocol, self.bytes)
    }
}

impl<C> ProtocolMessage<C>
where
    C: Clone,
{
    pub(crate) fn new_with_context(protocol: String, bytes: Vec<u8>, context: C) -> Self {
        ProtocolMessage {
            protocol,
            bytes,
            context,
        }
    }
}
