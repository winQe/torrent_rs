use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

use crate::message::PieceIndex;
use crate::piece::{BlockManager, PieceManager};

/// Thread-safe container for shared download state.
/// Uses RwLock for read-heavy data and Mutex for write-heavy data.
pub struct SharedState {
    /// Manages piece availability and rarest-first selection
    pub piece_manager: RwLock<PieceManager>,
    /// Manages block-level downloads within pieces
    pub block_manager: Mutex<BlockManager>,
    /// Set of pieces that have been verified and written to disk
    pub completed_pieces: RwLock<HashSet<PieceIndex>>,
    /// Download statistics
    pub stats: DownloadStats,
}

impl SharedState {
    pub fn new(total_pieces: u32, piece_size: u32) -> Arc<Self> {
        Arc::new(Self {
            piece_manager: RwLock::new(PieceManager::new(total_pieces, piece_size)),
            block_manager: Mutex::new(BlockManager::new()),
            completed_pieces: RwLock::new(HashSet::new()),
            stats: DownloadStats::new(total_pieces),
        })
    }
}

/// Atomic counters for download statistics.
/// All operations are lock-free for performance.
pub struct DownloadStats {
    /// Bytes downloaded so far
    downloaded_bytes: AtomicU64,
    /// Bytes uploaded so far
    uploaded_bytes: AtomicU64,
    /// Number of completed pieces
    pieces_completed: AtomicU64,
    /// Total number of pieces
    total_pieces: u32,
    /// When the download started
    start_time: Instant,
}

impl DownloadStats {
    pub fn new(total_pieces: u32) -> Self {
        Self {
            downloaded_bytes: AtomicU64::new(0),
            uploaded_bytes: AtomicU64::new(0),
            pieces_completed: AtomicU64::new(0),
            total_pieces,
            start_time: Instant::now(),
        }
    }

    pub fn add_downloaded(&self, bytes: u64) {
        self.downloaded_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn add_uploaded(&self, bytes: u64) {
        self.uploaded_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn increment_pieces(&self) {
        self.pieces_completed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn downloaded_bytes(&self) -> u64 {
        self.downloaded_bytes.load(Ordering::Relaxed)
    }

    pub fn uploaded_bytes(&self) -> u64 {
        self.uploaded_bytes.load(Ordering::Relaxed)
    }

    pub fn pieces_completed(&self) -> u64 {
        self.pieces_completed.load(Ordering::Relaxed)
    }

    pub fn total_pieces(&self) -> u32 {
        self.total_pieces
    }

    pub fn progress_percent(&self) -> f64 {
        if self.total_pieces == 0 {
            return 100.0;
        }
        (self.pieces_completed() as f64 / self.total_pieces as f64) * 100.0
    }

    /// Returns download speed in bytes per second.
    pub fn download_speed(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed < 0.001 {
            return 0.0;
        }
        self.downloaded_bytes() as f64 / elapsed
    }
}

/// A completed piece ready for verification and writing to disk.
pub struct CompletedPiece {
    pub index: PieceIndex,
    pub data: Vec<u8>,
}
