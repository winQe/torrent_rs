#![allow(dead_code)]

mod bitfield;
mod codec;
pub use bitfield::Bitfield;
pub use codec::MessageCodec;

pub type PieceIndex = u32;

#[derive(Debug, PartialEq)]
pub enum PeerMessage {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32), // Fixed length, 4 bytes for piece index should be enough
    Bitfield(Vec<u8>),
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
    Port(u16), // For newer versions that implements DHT, stored in 2 bytes
}

impl PeerMessage {
    pub fn message_id(&self) -> Option<u8> {
        match self {
            PeerMessage::KeepAlive => None, // KeepAlive has no ID
            PeerMessage::Choke => Some(0),
            PeerMessage::Unchoke => Some(1),
            PeerMessage::Interested => Some(2),
            PeerMessage::NotInterested => Some(3),
            PeerMessage::Have(_) => Some(4),
            PeerMessage::Bitfield(_) => Some(5),
            PeerMessage::Request { .. } => Some(6),
            PeerMessage::Piece { .. } => Some(7),
            PeerMessage::Cancel { .. } => Some(8),
            PeerMessage::Port(_) => Some(9),
        }
    }
}
