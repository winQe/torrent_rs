use std::net::SocketAddrV4;

mod address;
mod handshake;

#[derive(Debug, Clone, PartialEq)]
pub struct PeerAddresses(pub Vec<SocketAddrV4>);

enum PeerState {
    Choke,
    Unchoke,
}

pub struct Peer {
    addr: SocketAddrV4,
    state: PeerState,
    info_hash: String,
    peer_id: String,
}
