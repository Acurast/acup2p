use futures::Stream;

use super::types::StreamMessage;

pub trait InboundStream: Stream<Item = StreamMessage> {}
