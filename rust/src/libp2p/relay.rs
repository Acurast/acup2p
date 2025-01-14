use std::ops::{Deref, DerefMut};

use libp2p::Multiaddr;

use crate::types::ReconnectPolicy;

pub(super) struct Relay {
    addr: Multiaddr,
    status: Status,
}

pub(super) enum Status {
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

pub(super) enum ConnectionUpdate {
    SentObservedAddr,
    LearntObservedAddr(Multiaddr),
}

impl Relay {
    pub(super) fn new(addr: Multiaddr) -> Self {
        Relay {
            addr,
            status: Status::Disconnected(0),
        }
    }

    pub(super) fn set_unreachable(&mut self) {
        self.status = Status::Unreachable;
    }

    pub(super) fn set_disconnected(&mut self, reconn_policy: &ReconnectPolicy) {
        self.status = match reconn_policy {
            ReconnectPolicy::Never => Status::Unreachable,
            ReconnectPolicy::Attempts(max_attempts) => {
                let conn_attempts = match self.status {
                    Status::Disconnected(i) => i + 1,
                    _ => 1,
                };

                if conn_attempts < *max_attempts {
                    Status::Disconnected(conn_attempts)
                } else {
                    Status::Unreachable
                }
            }
            ReconnectPolicy::Always => Status::Disconnected(0),
        }
    }

    pub(super) fn set_connecting(&mut self) {
        self.status = Status::Connecting {
            told_observed_addr: false,
            learnt_observed_addr: false,
        };
    }

    pub(super) fn update_connecting(&mut self, update: ConnectionUpdate) {
        match self.status {
            Status::Connecting {
                told_observed_addr,
                learnt_observed_addr,
            } => {
                let relay = &self.addr;
                let (told_observed_addr, learnt_observed_addr) = match update {
                    ConnectionUpdate::SentObservedAddr => {
                        if !told_observed_addr {
                            tracing::info!(%relay, "told relay address");
                        }
                        (true, learnt_observed_addr)
                    }
                    ConnectionUpdate::LearntObservedAddr(multiaddr) => {
                        if !learnt_observed_addr {
                            tracing::info!(%relay, observed_addr=%multiaddr, "learnt observed address");
                        }
                        (told_observed_addr, true)
                    }
                };

                self.status = if told_observed_addr && learnt_observed_addr {
                    tracing::info!(%relay, "relay connection established");
                    Status::Connected
                } else {
                    Status::Connecting {
                        told_observed_addr,
                        learnt_observed_addr,
                    }
                };
            }
            _ => {}
        }
    }

    pub(super) fn set_pending_reservation(&mut self) {
        self.status = Status::PendingReservation;
    }

    pub(super) fn set_relaying(&mut self) {
        let relay = &self.addr;
        tracing::info!(%relay, "relay ready");
        self.status = Status::Relaying;
    }

    pub(super) fn is_unreachable(&self) -> bool {
        match self.status {
            Status::Unreachable => true,
            _ => false,
        }
    }

    pub(super) fn is_connected(&self) -> bool {
        match self.status {
            Status::Connected => true,
            _ => false,
        }
    }

    pub(super) fn is_relaying(&self) -> bool {
        match self.status {
            Status::Relaying => true,
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
