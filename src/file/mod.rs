#![allow(dead_code)]

use std::path::PathBuf;

use crate::message::PieceIndex;

pub trait FileManager: Sized {
    fn new(base_path: PathBuf, files: Vec<(String, u64)>, piece_size: u32) -> anyhow::Result<Self>;
    fn write_piece(&mut self, piece_index: PieceIndex, data: &[u8]) -> anyhow::Result<()>;
}

pub mod disk;
pub use disk::DiskFileManager;
