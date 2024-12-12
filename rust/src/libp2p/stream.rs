use crate::base;

pub enum StreamMessage {
    Frame(base::types::StreamMessage),
    EndOfStream,
}
