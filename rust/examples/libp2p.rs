use acup2p::base::types::{Event, Identity, NodeId, OutboundProtocolMessage};
use acup2p::types::connection::ReconnectPolicy;
use acup2p::utils::bytes::FitIntoArr;
use acup2p::{libp2p, Config, Node};
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
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let opt = Opt::parse();
    let config = Config {
        msg_protocols: vec![PROTOCOL_ECHO],
        ..opt.into_config(Some(Default::default()))
    };

    let mut node = libp2p::Node::new(config).await?;
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                on_stdin_line(&mut node, line).await;
            }
            event = node.next() => match event {
                Some(event) => on_node_event(&mut node, event).await,
                None => break,
            }
        }
    }

    Ok(())
}

async fn on_stdin_line<T>(node: &mut T, line: String)
where
    T: Node,
{
    match Command::try_from(line + "\n") {
        Ok(Command::Close) => {
            if let Err(e) = node.close().await {
                println!("error: {e:?}")
            }
        }
        Ok(Command::Dial { node: peer }) => {
            if let Err(e) = node.connect(&[peer]).await {
                println!("error: {e:?}")
            }
        }
        Ok(Command::Echo {
            node: receiver,
            message,
        }) => {
            let request = OutboundProtocolMessage::new_request(
                PROTOCOL_ECHO.to_string(),
                message.as_bytes().to_vec(),
            );
            if let Err(e) = node.send_message(request, &[receiver]).await {
                println!("error: {e:?}")
            }
        }
        _ => {}
    }
}

async fn on_node_event<T>(node: &mut T, event: Event)
where
    T: Node,
{
    match event {
        Event::ListeningOn { address } => {
            println!("listening on {address}");
        }
        Event::Connected { node } => {
            println!("{node:?} connected");
        }
        Event::Disconnected { node } => {
            println!("{node:?} disconnected");
        }
        Event::InboundRequest { sender, request } => match request.protocol.as_str() {
            PROTOCOL_ECHO => match str::from_utf8(&request.bytes) {
                Ok(text) => {
                    println!("echo request from {sender:?} ({text})");
                    let response = request.bytes.clone();
                    let response = OutboundProtocolMessage::new_response(request, response);
                    if let Err(err) = node.send_message(response, &[sender]).await {
                        println!("error: {err:?}");
                    };
                }
                Err(err) => {
                    println!("invalid echo request from {sender:?} ({err})");
                }
            },
            _ => {
                println!("message from {sender:?} ({})", request.protocol)
            }
        },
        Event::InboundResponse { sender, response } => match response.protocol.as_str() {
            PROTOCOL_ECHO => match str::from_utf8(&response.bytes) {
                Ok(text) => {
                    println!("echo response from {sender:?} ({text})");
                }
                Err(err) => {
                    println!("invalid echo response from {sender:?} ({err})");
                }
            },
            _ => {
                println!("message from {sender:?} ({})", response.protocol)
            }
        },
        Event::OutboundRequest { receiver, request } => match request.protocol.as_str() {
            PROTOCOL_ECHO => match str::from_utf8(&request.bytes) {
                Ok(text) => {
                    println!("echo request sent to {receiver:?} ({text})");
                }
                Err(err) => {
                    println!("invalid echo request to {receiver:?} ({err})");
                }
            },
            _ => {
                println!("message to {receiver:?} ({})", request.protocol)
            }
        },
        Event::OutboundResponse { receiver, response } => match response.protocol.as_str() {
            PROTOCOL_ECHO => match str::from_utf8(&response.bytes) {
                Ok(text) => {
                    println!("echo response sent to {receiver:?} ({text})");
                }
                Err(err) => {
                    println!("invalid echo response to {receiver:?} ({err})");
                }
            },
            _ => {
                println!("message to {receiver:?} ({})", response.protocol)
            }
        },
        Event::Error { cause } => {
            println!("error: {cause}")
        }
        _ => {}
    }
}

#[derive(Debug)]
enum Command {
    Close,
    Dial { node: NodeId },
    Echo { node: NodeId, message: String },
}

impl TryFrom<String> for Command {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        fn node_parser(input: &str) -> IResult<&str, NodeId> {
            alt((
                map(
                    delimited(tag("peer("), take_until(")"), tag(")")),
                    |peer_id: &str| NodeId::Peer {
                        peer_id: peer_id.to_string(),
                    },
                ),
                map(
                    delimited(tag("addr("), take_until(")"), tag(")")),
                    |addr: &str| NodeId::Address {
                        address: addr.to_string(),
                    },
                ),
            ))(input)
        }

        if value.starts_with("close") {
            return Ok(Command::Close);
        } else if value.starts_with("dial") {
            let (_, node) =
                preceded(tag("dial:"), node_parser)(value.as_str()).map_err(|e| e.to_string())?;

            return Ok(Command::Dial { node });
        } else if value.starts_with("echo") {
            let (_, (node, message)) = preceded(
                tag("echo:"),
                separated_pair(node_parser, tag(":"), take_until("\n")),
            )(value.as_str())
            .map_err(|e| e.to_string())?;

            return Ok(Command::Echo {
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

impl Opt {
    fn into_config<'a, L>(&'a self, log: L) -> Config<'a, L>
    where
        L: Default,
    {
        let default = Config::<L>::default();

        let identity = if let Some(seed) = &self.identity_seed {
            Identity::Seed(seed.clone().fit_into_arr())
        } else {
            default.identity
        };

        let relay_addrs = if let Some(addrs) = &self.relay_addrs {
            addrs.iter().map(|s| s.as_str()).collect::<Vec<_>>()
        } else {
            default.relay_addrs
        };

        let reconn_policy = if let Some(n) = self.max_relay_reconn_attempts {
            ReconnectPolicy::Attempts(n)
        } else {
            default.reconn_policy
        };

        let idle_conn_timeout = if let Some(n) = self.idle_conn_timeout {
            Duration::from_secs(n)
        } else {
            default.idle_conn_timeout
        };

        Config {
            identity,
            relay_addrs,
            reconn_policy,
            idle_conn_timeout,
            log,
            ..Default::default()
        }
    }
}
