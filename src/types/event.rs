use crate::types::jsonrpc;

pub enum Event {
  ListeningOn(String),

  Connected(String),
  Disconnected(String),

  Data(Data),
}

pub enum Data {
  Message(jsonrpc::Request),
}

