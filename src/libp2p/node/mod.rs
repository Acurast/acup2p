use std::collections::HashMap;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use libp2p::core::transport::ListenerId;
use libp2p::dns::{ResolverConfig, ResolverOpts};
use libp2p::{noise, tcp, tls, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder};
use libp2p_request_response::{InboundRequestId, ResponseChannel};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::{select, spawn};

use crate::base::node;
use crate::connection::ReconnectPolicy;
use crate::libp2p::behaviour::Behaviour;
use crate::libp2p::context::Context;
use crate::libp2p::identity::ed25519;
use crate::libp2p::node::intent::Intent;
use crate::libp2p::node::message::Message;
use crate::libp2p::relay::Relay;
use crate::types;
use crate::types::event::Event;
use crate::types::identity::Identity;
use crate::types::message::OutboundProtocolMessage;
use crate::types::result::Result;
use crate::types::transform::MaybeInto;

use super::peer::Id;

mod dial;
mod event;
mod intent;
mod listen;
mod message;
mod send;
mod swarm_event;

const DEFAULT_CHANNEL_BUFFER: u8 = 255;

pub struct Node {
    intent_tx: Sender<Intent>,
    event_rx: Receiver<Event<Context>>,
}

struct NodeInner {
    ext_event_tx: Sender<Event<Context>>,
    ext_intent_rx: Receiver<Intent>,

    self_msg_tx: Sender<Message>,
    self_msg_rx: Receiver<Message>,

    is_active: bool,
    swarm: Swarm<Behaviour>,

    listeners: HashMap<ListenerId, u8>,
    relays: HashMap<PeerId, Relay>,
    response_channels: HashMap<(Id, String, InboundRequestId), ResponseChannel<Vec<u8>>>,

    reconn_policy: ReconnectPolicy,
}

impl NodeInner {
    async fn new(
        event_tx: Sender<Event<Context>>,
        intent_rx: Receiver<Intent>,
        config: node::Config<'_>,
    ) -> Result<Self> {
        let security_upgrade = (tls::Config::new, noise::Config::new);

        let builder = match config.identity {
            Identity::Seed(seed) => SwarmBuilder::with_existing_identity(ed25519::generate(seed)?),
            Identity::Random => SwarmBuilder::with_new_identity(),
        };

        let swarm = builder
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                security_upgrade,
                yamux::Config::default,
            )?
            .with_quic()
            .with_dns_config(ResolverConfig::default(), ResolverOpts::default())
            .with_websocket(security_upgrade, yamux::Config::default)
            .await?
            .with_relay_client(security_upgrade, yamux::Config::default)?
            .with_behaviour(|key, relay_behaviour| {
                Ok(Behaviour::new(key, relay_behaviour, &config.msg_protocols)?)
            })?
            .with_swarm_config(|c| c.with_idle_connection_timeout(config.idle_conn_timeout))
            .build();

        let (int_event_tx, int_event_rx) = channel(usize::from(DEFAULT_CHANNEL_BUFFER));

        Ok(NodeInner {
            ext_event_tx: event_tx,
            ext_intent_rx: intent_rx,

            self_msg_tx: int_event_tx,
            self_msg_rx: int_event_rx,

            is_active: true,
            swarm,

            listeners: HashMap::new(),
            relays: config
                .relay_addrs
                .iter()
                .filter_map(|addr| {
                    let addr = if let Ok(addr) = addr.parse::<Multiaddr>() {
                        addr
                    } else {
                        return None;
                    };

                    let peer_id: PeerId = if let Some(peer_id) = addr.clone().maybe_into() {
                        peer_id
                    } else {
                        return None;
                    };

                    let relay = Relay::new(addr);

                    Some((peer_id, relay))
                })
                .collect(),
            response_channels: HashMap::new(),

            reconn_policy: config.reconn_policy,
        })
    }

    async fn start(&mut self) {
        if let Err(e) = self.listen() {
            self.notify_error(e.to_string()).await;
            return;
        }

        let mut swarm_closed = false;
        let mut cmd_closed = false;
        let mut int_event_closed = false;

        loop {
            select! {
                event = self.swarm.next() => {
                    if let Some(event) = event {
                        self.on_swarm_event(event).await;
                    } else {
                        swarm_closed = true;
                    }
                }
                intent = self.ext_intent_rx.recv() => {
                    if let Some(intent) = intent {
                        self.on_intent(intent).await;
                    } else {
                        cmd_closed = true;
                    }
                }
                event = self.self_msg_rx.recv() => {
                    if let Some(event) = event {
                        self.on_self_message(event).await;
                    } else {
                        int_event_closed = true;
                    }
                }
            }

            if (swarm_closed || !self.is_active) && cmd_closed && int_event_closed {
                break;
            }
        }

        tracing::info!("finished");
    }
}

impl Stream for Node {
    type Item = Event<Context>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.as_mut().event_rx.poll_recv(cx)
    }
}

#[async_trait]
impl node::Node for Node {
    type Context = Context;

    async fn new(config: node::Config<'_>) -> Result<Self> {
        let (event_tx, event_rx) = channel(usize::from(DEFAULT_CHANNEL_BUFFER));
        let (intent_tx, intent_rx) = channel(usize::from(DEFAULT_CHANNEL_BUFFER));

        let mut inner = NodeInner::new(event_tx, intent_rx, config).await?;
        spawn(async move {
            inner.start().await;
        });

        Ok(Node {
            intent_tx,
            event_rx,
        })
    }

    async fn connect(&mut self, nodes: &[types::node::Id]) -> Result<()> {
        for node in nodes {
            self.intent_tx.send(Intent::Dial(node.try_into()?)).await?;
        }

        Ok(())
    }

    async fn disconnect(&mut self, nodes: &[types::node::Id]) -> Result<()> {
        for node in nodes {
            self.intent_tx
                .send(Intent::Disconnect(node.try_into()?))
                .await?;
        }

        Ok(())
    }

    async fn send_message(
        &mut self,
        message: OutboundProtocolMessage<Context>,
        nodes: &[types::node::Id],
    ) -> Result<()> {
        for node in nodes {
            self.intent_tx
                .send(Intent::DirectMessage {
                    peer: node.try_into()?,
                    message: message.clone(),
                })
                .await?;
        }

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.intent_tx.send(Intent::Close).await?;
        self.event_rx.close();

        Ok(())
    }
}
