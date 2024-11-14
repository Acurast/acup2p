use crate::types::message::Message;

pub enum Event {
    ListeningOn(String),

    Connected(String),
    Disconnected(String),

    Data(Data),
}

pub enum Data {
    Message(Message),
}
