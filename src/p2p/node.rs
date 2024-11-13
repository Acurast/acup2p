use crate::types::{event::Event, identity::Identity, jsonrpc::Message, result::Result};
use futures::Stream;
use std::{future::Future, time::Duration};

pub trait Node: Stream<Item = Event> {
    fn new(config: Config) -> impl Future<Output = Result<Self>> + Send
    where
        Self: Sized;

    fn connect(&mut self, addrs: &[&str]) -> impl Future<Output = Result<()>> + Send;
    fn disonnect(&mut self, addrs: &[&str]) -> impl Future<Output = Result<()>> + Send;

    fn send(&mut self, message: Message) -> impl Future<Output = Result<()>> + Send;

    fn close(&mut self) -> impl Future<Output = Result<()>> + Send;
}

pub struct Config<'a> {
    pub identity: Identity,
    pub relay_addrs: &'a [&'a str],
    pub idle_conn_timeout: Duration,
}

impl Default for Config<'_> {
    fn default() -> Self {
        Self {
            identity: Identity::Random,
            relay_addrs: &[],
            idle_conn_timeout: Duration::ZERO,
        }
    }
}
