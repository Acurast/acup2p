use acurast_p2p_node::connection::ReconnectPolicy;
use acurast_p2p_node::event::Event;
use acurast_p2p_node::libp2p::builder::Logger;
use acurast_p2p_node::types::identity::Identity;
use acurast_p2p_node::types::message::OutboundProtocolMessage;
use acurast_p2p_node::{node, Config, Node, NodeBuilder};
use clap::Parser;
use core::str;
use futures::StreamExt;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::combinator::map;
use nom::sequence::{delimited, preceded, separated_pair};
use nom::IResult;
use std::error::Error;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::{io, select};

const PROTOCOL_ECHO: &str = "/echo/1";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();
    let config = Config {
        msg_protocols: vec![PROTOCOL_ECHO],
        ..Config::from(&opt)
    };
    let mut node = NodeBuilder::libp2p()
        .with_config(config)
        .with_logger(Logger::default())?
        .build()
        .await?;

    let mut stdin = io::BufReader::new(io::stdin()).lines();

    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                on_stdin_line(&mut node, line).await;
            }
            Some(event) = node.next() => {
                on_node_event(&mut node, event).await;
            }

        }
    }
}

async fn on_stdin_line<T>(node: &mut T, line: String)
where
    T: Node,
{
    match Command::try_from(line + "\n") {
        Ok(Command::Close) => {
            if let Err(e) = node.close().await {
                println!("error: {e:?}")
            };
        }
        Ok(Command::Message {
            node: receiver,
            message,
        }) => {
            let request = OutboundProtocolMessage::new_request(
                PROTOCOL_ECHO.to_string(),
                message.as_bytes().to_vec(),
            );
            if let Err(e) = node.send_message(request, &[receiver]).await {
                println!("error: {e:?}")
            };
        }
        _ => {}
    }
}

async fn on_node_event<T>(node: &mut T, event: Event<T::Context>)
where
    T: Node,
{
    match event {
        Event::ListeningOn(addr) => {
            println!("listening on {addr}");
        }
        Event::Connected(addr) => {
            println!("{addr:?} connected");
        }
        Event::Disconnected(addr) => {
            println!("{addr:?} disconnected");
        }
        Event::InboundRequest(sender, message) => match message.protocol.as_str() {
            PROTOCOL_ECHO => match str::from_utf8(&message.bytes) {
                Ok(text) => {
                    println!("echo request from {sender:?} ({text})");
                    let response = message.bytes.clone();
                    let response = OutboundProtocolMessage::new_response(message, response);
                    if let Err(err) = node.send_message(response, &[sender]).await {
                        println!("error: {err:?}");
                    };
                }
                Err(err) => {
                    println!("invalid echo request from {sender:?} ({err})");
                }
            },
            _ => {
                println!("message from {sender:?} ({})", message.protocol)
            }
        },
        Event::InboundResponse(sender, message) => match message.protocol.as_str() {
            PROTOCOL_ECHO => match str::from_utf8(&message.bytes) {
                Ok(text) => {
                    println!("echo response from {sender:?} ({text})");
                }
                Err(err) => {
                    println!("invalid echo response from {sender:?} ({err})");
                }
            },
            _ => {
                println!("message from {sender:?} ({})", message.protocol)
            }
        },
        Event::OutboundRequest(receiver, message) => match message.protocol.as_str() {
            PROTOCOL_ECHO => match str::from_utf8(&message.bytes) {
                Ok(text) => {
                    println!("echo request sent to {receiver:?} ({text})");
                }
                Err(err) => {
                    println!("invalid echo request to {receiver:?} ({err})");
                }
            },
            _ => {
                println!("message to {receiver:?} ({})", message.protocol)
            }
        },
        Event::OutboundResponse(receiver, message) => match message.protocol.as_str() {
            PROTOCOL_ECHO => match str::from_utf8(&message.bytes) {
                Ok(text) => {
                    println!("echo response sent to {receiver:?} ({text})");
                }
                Err(err) => {
                    println!("invalid echo response to {receiver:?} ({err})");
                }
            },
            _ => {
                println!("message to {receiver:?} ({})", message.protocol)
            }
        },
        Event::Error(msg) => {
            println!("error: {msg}")
        }
    }
}

#[derive(Debug)]
enum Command {
    Close,
    Message { node: node::Id, message: String },
}

impl TryFrom<String> for Command {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.starts_with("close") {
            return Ok(Command::Close);
        } else if value.starts_with("msg") {
            fn node_parser(input: &str) -> IResult<&str, node::Id> {
                alt((
                    map(
                        delimited(tag("peer("), take_until(")"), tag(")")),
                        |peer_id: &str| node::Id::Peer(peer_id.to_string()),
                    ),
                    map(
                        delimited(tag("addr("), take_until(")"), tag(")")),
                        |addr: &str| node::Id::Addr(addr.to_string()),
                    ),
                ))(input)
            }

            let (_, (node, message)) = preceded(
                tag("msg:"),
                separated_pair(node_parser, tag(":"), take_until("\n")),
            )(value.as_str())
            .map_err(|e| e.to_string())?;

            return Ok(Command::Message {
                node,
                message: message.to_string(),
            });
        }

        Err("unknown command".to_string())
    }
}

#[derive(Debug, Parser)]
struct Opt {
    #[clap(long, value_delimiter = ' ', num_args = 1..=32)]
    identity_seed: Option<Vec<u8>>,

    #[clap(long, value_delimiter = ' ', num_args = 1..)]
    relay_addrs: Option<Vec<String>>,

    #[clap(long)]
    max_relay_reconn_attempts: Option<u8>,

    #[clap(long)]
    idle_conn_timeout: Option<u64>,
}

impl<'a> From<&'a Opt> for Config<'a> {
    fn from(value: &'a Opt) -> Self {
        let default = Config::default();

        let identity = if let Some(seed) = &value.identity_seed {
            let mut seed = seed.clone();
            if seed.len() < 32 {
                let mut padded_seed = vec![0u8; 32 - seed.len()];
                padded_seed.extend(seed);

                seed = padded_seed;
            } else if seed.len() > 32 {
                seed.truncate(32);
            }

            let mut arr = [0u8; 32];
            arr.copy_from_slice(&seed);

            Identity::Seed(arr)
        } else {
            default.identity
        };

        let relay_addrs = if let Some(addrs) = &value.relay_addrs {
            addrs.iter().map(|s| s.as_str()).collect::<Vec<_>>()
        } else {
            default.relay_addrs
        };

        let reconn_policy = if let Some(n) = value.max_relay_reconn_attempts {
            ReconnectPolicy::Attempts(n)
        } else {
            default.reconn_policy
        };

        let idle_conn_timeout = if let Some(n) = value.idle_conn_timeout {
            Duration::from_secs(n)
        } else {
            default.idle_conn_timeout
        };

        Config {
            identity,
            relay_addrs,
            reconn_policy,
            idle_conn_timeout,
            ..Default::default()
        }
    }
}
