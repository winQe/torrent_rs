use crate::message::PieceIndex;

// 16 KB standard block size from https://wiki.theory.org/BitTorrentSpecification#Peer_wire_protocol_.28TCP.29
pub const BLOCK_SIZE: u32 = 16384;

pub type Block = Vec<u8>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockInfo {
    pub piece_index: PieceIndex,
    pub offset: u32,
    pub length: u32,
}

pub mod block_manager;
pub mod piece_manager;
pub mod verify;

pub use block_manager::BlockManager;
pub use piece_manager::PieceManager;
pub use verify::verify_piece;
