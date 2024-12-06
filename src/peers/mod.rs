#![allow(dead_code)]
use std::net::SocketAddrV4;

mod address;
mod handshake;

#[derive(Debug, Clone, PartialEq)]
pub struct PeerAddresses(pub Vec<SocketAddrV4>);

// To make it more readable
impl PeerAddresses {
    pub fn iter(&self) -> std::slice::Iter<'_, SocketAddrV4> {
        self.0.iter()
    }
}

#[derive(Debug)]
enum PeerState {
    Choke,
    Unchoke,
}

#[derive(Debug)]
pub struct Peer {
    addr: SocketAddrV4,
    state: PeerState,
    info_hash: [u8; 20],
    peer_id: String,
}

impl Peer {
    pub fn new(address: SocketAddrV4, info_hash: [u8; 20], peer_id: String) -> Self {
        Self {
            addr: address,
            info_hash,
            peer_id,
            state: PeerState::Choke,
        }
    }
}
