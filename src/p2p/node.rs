use crate::types::{event::Event, identity::Identity, message::Message, result::Result};
use async_trait::async_trait;
use futures::Stream;
use std::time::Duration;

#[async_trait]
pub trait Node: Stream<Item = Event> {
    async fn new(config: Config) -> Result<Self>
    where
        Self: Sized;

    async fn connect(&mut self, addrs: &[&str]) -> Result<()>;
    async fn disonnect(&mut self, addrs: &[&str]) -> Result<()>;

    async fn send(&mut self, message: Message, receivers: &[Reciever]) -> Result<()>;

    async fn close(&mut self) -> Result<()>;
}

pub struct Config<'a> {
    pub identity: Identity,
    pub msg_protocols: &'a [&'a str],
    pub relay_addrs: &'a [&'a str],
    pub idle_conn_timeout: Duration,
}

impl Default for Config<'_> {
    fn default() -> Self {
        Self {
            identity: Identity::Random,
            msg_protocols: &[],
            relay_addrs: &[],
            idle_conn_timeout: Duration::ZERO,
        }
    }
}

pub enum Reciever<'a> {
    Peer(&'a str),
    Addr(&'a str),
    Topic(&'a str),
}
