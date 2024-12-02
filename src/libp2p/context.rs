use libp2p_request_response::{InboundRequestId, OutboundRequestId};

use crate::context;

#[derive(Debug, Clone, Copy)]
pub struct Context {}

impl context::Context for Context {
    type InboundRequest = InboundRequestId;
    type InboundResponse = OutboundRequestId;
}
