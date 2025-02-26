pub(super) mod dial;
pub(super) mod event;
pub(super) mod intent;
pub(super) mod listen;
pub(super) mod message;
pub(super) mod send;
pub(super) mod stream;
pub(super) mod swarm_event;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use futures::StreamExt;
use libp2p::core::transport::ListenerId;
use libp2p::identity::Keypair;
use libp2p::{noise, tcp, tls, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder};
use libp2p_request_response as request_response;
use stream::StreamControl;
use tokio::select;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;

use crate::base::types::{Event, Identity};
use crate::base::{self};
use crate::types::{MaybeInto, ReconnectPolicy, Result};

use super::behaviour::Behaviour;
use super::identity::ed25519;
use super::node::NodeId;
use super::dcutr::Relay;
use super::Intent;

use self::listen::ListenerType;
use self::message::Message;

const DEFAULT_CHANNEL_BUFFER: usize = 255;

pub(super) struct NodeInner {
    ext_event_tx: Sender<Event>,
    ext_intent_rx: Receiver<Intent>,

    self_msg_tx: Sender<Message>,
    self_msg_rx: Receiver<Message>,

    is_active: bool,
    swarm: Swarm<Behaviour>,

    streams: HashMap<Arc<String>, StreamControl>,

    required_listeners: HashSet<ListenerType>,
    tracked_listeners: HashMap<ListenerId, ListenerType>,

    listeners: HashSet<ListenerId>,
    relays: HashMap<PeerId, Relay>,
    response_channels:
        HashMap<(NodeId, String, String), request_response::ResponseChannel<Vec<u8>>>,

    reconn_policy: ReconnectPolicy,
}

impl NodeInner {
    pub(super) async fn new<L>(
        event_tx: Sender<Event>,
        intent_rx: Receiver<Intent>,
        config: &base::Config<'_, L>,
    ) -> Result<Self> {
        let security_upgrade = (tls::Config::new, noise::Config::new);

        let builder = match config.identity {
            Identity::Random => SwarmBuilder::with_new_identity(),
            Identity::Seed(seed) => SwarmBuilder::with_existing_identity(ed25519::generate(seed)?),
            Identity::Keypair(secret_key) => {
                let keypair = match secret_key {
                    base::types::SecretKey::Ed25519(secret_key) => {
                        Keypair::ed25519_from_bytes(secret_key)?
                    }
                };

                SwarmBuilder::with_existing_identity(keypair)
            }
        };

        let builder = builder
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                security_upgrade,
                yamux::Config::default,
            )?
            .with_quic();

        #[cfg(any(target_os = "android"))]
        let builder = builder
            .with_dns_config(
                libp2p::dns::ResolverConfig::default(),
                libp2p::dns::ResolverOpts::default(),
            )
            .with_websocket_custom(security_upgrade, yamux::Config::default)
            .await?;

        #[cfg(not(any(target_os = "android")))]
        let builder = builder
            .with_dns()?
            .with_websocket(security_upgrade, yamux::Config::default)
            .await?;

        let swarm = builder
            .with_relay_client(security_upgrade, yamux::Config::default)?
            .with_behaviour(|key, relay_behaviour| {
                Ok(Behaviour::new(key, relay_behaviour, &config.msg_protocols)?)
            })?
            .with_swarm_config(|c| c.with_idle_connection_timeout(config.idle_conn_timeout))
            .build();

        let streams = config
            .stream_protocols
            .iter()
            .map(|&p| {
                let protocol = Arc::new(p.to_owned());
                let control = StreamControl::new(protocol.clone(), &swarm.behaviour().stream)?;

                Ok((protocol, control))
            })
            .collect::<Result<_, stream::Error>>()?;

        let (int_event_tx, int_event_rx) = channel(DEFAULT_CHANNEL_BUFFER);

        Ok(NodeInner {
            ext_event_tx: event_tx,
            ext_intent_rx: intent_rx,

            self_msg_tx: int_event_tx,
            self_msg_rx: int_event_rx,

            is_active: true,
            swarm,

            streams,

            required_listeners: HashSet::new(),
            tracked_listeners: HashMap::new(),

            listeners: HashSet::new(),
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

    pub(super) async fn start(
        &mut self,
        incoming_stream_tx: &HashMap<
            Arc<String>,
            Arc<Mutex<Sender<(base::types::NodeId, Box<dyn base::stream::IncomingStream>)>>>,
        >,
    ) {
        if let Err(e) = self.listen() {
            self.notify_error(e.to_string()).await;
            return;
        }

        self.subscribe_incoming_streams(incoming_stream_tx);

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
