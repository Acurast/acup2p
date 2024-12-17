use crate::base;

use super::node::NodeId;

pub enum StreamMessage {
    Frame((NodeId, base::types::StreamMessage)),
    EOS(NodeId), /* End Of Stream */
}
