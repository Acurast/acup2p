use std::ops::{Deref, DerefMut};

use libp2p::Multiaddr;

use crate::types::ReconnectPolicy;

pub(super) struct Relay {
    addr: Multiaddr,
    status: RelayStatus,
}

pub(super) enum RelayStatus {
    Unreachable,
    Disconnected(u8),
    Connecting {
        told_observed_addr: bool,
        learnt_observed_addr: bool,
    },
    Connected,
    PendingReservation,
    Relaying,
}

pub(super) enum RelayConnectionUpdate {
    SentObservedAddr,
    LearntObservedAddr(Multiaddr),
}

impl Relay {
    pub(super) fn new(addr: Multiaddr) -> Self {
        Relay {
            addr,
            status: RelayStatus::Disconnected(0),
        }
    }

    pub(super) fn set_unreachable(&mut self) {
        self.status = RelayStatus::Unreachable;
    }

    pub(super) fn set_disconnected(&mut self, reconn_policy: &ReconnectPolicy) {
        self.status = match reconn_policy {
            ReconnectPolicy::Never => RelayStatus::Unreachable,
            ReconnectPolicy::Attempts(max_attempts) => {
                let conn_attempts = match self.status {
                    RelayStatus::Disconnected(i) => i + 1,
                    _ => 1,
                };

                if conn_attempts < *max_attempts {
                    RelayStatus::Disconnected(conn_attempts)
                } else {
                    RelayStatus::Unreachable
                }
            }
            ReconnectPolicy::Always => RelayStatus::Disconnected(0),
        }
    }

    pub(super) fn set_connecting(&mut self) {
        self.status = RelayStatus::Connecting {
            told_observed_addr: false,
            learnt_observed_addr: false,
        };
    }

    pub(super) fn update_connecting(&mut self, update: RelayConnectionUpdate) {
        match self.status {
            RelayStatus::Connecting {
                told_observed_addr,
                learnt_observed_addr,
            } => {
                let relay = &self.addr;
                let (told_observed_addr, learnt_observed_addr) = match update {
                    RelayConnectionUpdate::SentObservedAddr => {
                        if !told_observed_addr {
                            tracing::info!(%relay, "told relay address");
                        }
                        (true, learnt_observed_addr)
                    }
                    RelayConnectionUpdate::LearntObservedAddr(multiaddr) => {
                        if !learnt_observed_addr {
                            tracing::info!(%relay, observed_addr=%multiaddr, "learnt observed address");
                        }
                        (told_observed_addr, true)
                    }
                };

                self.status = if told_observed_addr && learnt_observed_addr {
                    tracing::info!(%relay, "relay connection established");
                    RelayStatus::Connected
                } else {
                    RelayStatus::Connecting {
                        told_observed_addr,
                        learnt_observed_addr,
                    }
                };
            }
            _ => {}
        }
    }

    pub(super) fn set_pending_reservation(&mut self) {
        self.status = RelayStatus::PendingReservation;
    }

    pub(super) fn set_relaying(&mut self) {
        let relay = &self.addr;
        tracing::info!(%relay, "relay ready");
        self.status = RelayStatus::Relaying;
    }

    pub(super) fn is_unreachable(&self) -> bool {
        match self.status {
            RelayStatus::Unreachable => true,
            _ => false,
        }
    }

    pub(super) fn is_connected(&self) -> bool {
        match self.status {
            RelayStatus::Connected => true,
            _ => false,
        }
    }

    pub(super) fn is_relaying(&self) -> bool {
        match self.status {
            RelayStatus::Relaying => true,
            _ => false,
        }
    }
}

impl Deref for Relay {
    type Target = Multiaddr;

    fn deref(&self) -> &Self::Target {
        &self.addr
    }
}

impl DerefMut for Relay {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.addr
    }
}

pub(super) enum ConnectionStatus {
    Circuit(Multiaddr),
    Direct(Multiaddr),
}

impl ConnectionStatus {
    pub(super) fn address(&self) -> &Multiaddr {
        match self {
            ConnectionStatus::Circuit(addr) => addr,
            ConnectionStatus::Direct(addr) => addr,
        }
    }
}