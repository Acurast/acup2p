use std::ops::{Deref, DerefMut};

use libp2p::Multiaddr;

use crate::types::ReconnectPolicy;

pub(crate) struct Relay {
    addr: Multiaddr,
    status: Status,
}

pub(crate) enum Status {
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

impl Relay {
    pub(crate) fn new(addr: Multiaddr) -> Self {
        Relay {
            addr,
            status: Status::Disconnected(0),
        }
    }

    pub(crate) fn set_unreachable(&mut self) {
        self.status = Status::Unreachable;
    }

    pub(crate) fn set_disconnected(&mut self, reconn_policy: &ReconnectPolicy) {
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

    pub(crate) fn set_connecting(&mut self) {
        self.status = Status::Connecting {
            told_observed_addr: false,
            learnt_observed_addr: false,
        };
    }

    pub(crate) fn update_connecting(
        &mut self,
        told_observed_addr: bool,
        learnt_observed_addr: bool,
    ) {
        match self.status {
            Status::Connecting {
                told_observed_addr: curr_told_observed_addr,
                learnt_observed_addr: curr_learnt_observed_addr,
            } => {
                let told_observed_addr = told_observed_addr || curr_told_observed_addr;
                let learnt_observed_addr = learnt_observed_addr || curr_learnt_observed_addr;

                self.status = if told_observed_addr && learnt_observed_addr {
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

    pub(crate) fn set_pending_reservation(&mut self) {
        self.status = Status::PendingReservation;
    }

    pub(crate) fn set_relaying(&mut self) {
        self.status = Status::Relaying;
    }

    pub(crate) fn is_unreachable(&self) -> bool {
        match self.status {
            Status::Unreachable => true,
            _ => false,
        }
    }

    pub(crate) fn is_connected(&self) -> bool {
        match self.status {
            Status::Connected => true,
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
