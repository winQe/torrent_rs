#![allow(dead_code)]

use crate::message::PieceIndex;

trait FileManager: Sized {
    fn new(files: Vec<(String, u64)>, piece_size: u32) -> anyhow::Result<Self>;
    fn write_piece(&mut self, piece_index: PieceIndex, data: &[u8]) -> anyhow::Result<()>;
}

pub mod disk;
