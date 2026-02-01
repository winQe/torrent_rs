#![allow(dead_code)]
use std::collections::{BTreeSet, HashMap, HashSet};

use crate::message::Bitfield;
use crate::message::PieceIndex;

// TODO: Make this thread safe
#[derive(Debug)]
pub struct PieceManager {
    // Tracks number of peers that have each piece (updated dynamically)
    piece_counts: HashMap<PieceIndex, u32>,
    // Ordered set of (availability, index) for efficient rarest-first selection
    availability_queue: BTreeSet<(u32, PieceIndex)>,
    // Pieces we've successfully downloaded and verified
    completed: HashSet<PieceIndex>,
    // Pieces currently being downloaded
    pending: HashSet<PieceIndex>,
    // Total pieces in torrent
    total_pieces: u32,
    // Standard piece size (last piece may be smaller)
    piece_size: u32,
}

impl PieceManager {
    pub fn new(total_pieces: u32, piece_size: u32) -> Self {
        Self {
            piece_counts: HashMap::new(),
            availability_queue: BTreeSet::new(),
            completed: HashSet::new(),
            pending: HashSet::new(),
            total_pieces,
            piece_size,
        }
    }

    /// Update availability when peer connects with their bitfield
    pub fn add_peer(&mut self, bitfield: &Bitfield) {
        for piece_index in bitfield.iter() {
            if self.completed.contains(&piece_index) {
                continue;
            }

            let entry = self.piece_counts.entry(piece_index).or_insert(0);
            let old_count = *entry;
            *entry += 1;

            // Need to remove the old
            if old_count > 0 {
                self.availability_queue.remove(&(old_count, piece_index));
            }
            self.availability_queue.insert((*entry, piece_index));
        }
    }

    /// Select next piece to download using rarest-first strategy
    pub fn next_piece(&mut self) -> Option<PieceIndex> {
        // Find first available piece that's not completed or pending
        let candidate = self
            .availability_queue
            .iter()
            .find(|&&(count, piece)| {
                count > 0 && !self.completed.contains(&piece) && !self.pending.contains(&piece)
            })
            .copied();

        if let Some((_, piece)) = candidate {
            self.pending.insert(piece);
            Some(piece)
        } else {
            None
        }
    }

    /// Mark piece as successfully downloaded
    pub fn mark_completed(&mut self, piece: PieceIndex) {
        self.pending.remove(&piece);
        self.completed.insert(piece);
        self.piece_counts.remove(&piece);
        self.availability_queue.retain(|&(_, p)| p != piece);
    }

    /// Handle peer disconnection (update availability counts)
    pub fn remove_peer(&mut self, bitfield: &Bitfield) {
        for piece_index in bitfield.iter() {
            if let Some(count) = self.piece_counts.get_mut(&piece_index) {
                let old_count = *count;
                *count = count.saturating_sub(1);
                self.availability_queue.remove(&(old_count, piece_index));
                if *count > 0 {
                    self.availability_queue.insert((*count, piece_index));
                }
            }
        }
    }

    /// Mark piece as failed (e.g., hash verification failed).
    /// This removes it from pending so it can be re-requested.
    pub fn mark_failed(&mut self, piece: PieceIndex) {
        self.pending.remove(&piece);
    }

    /// Check if all pieces have been downloaded
    pub fn is_complete(&self) -> bool {
        self.completed.len() == self.total_pieces as usize
    }

    /// Get download progress as (completed, total)
    pub fn progress(&self) -> (usize, u32) {
        (self.completed.len(), self.total_pieces)
    }

    /// Get the standard piece size
    pub fn piece_size(&self) -> u32 {
        self.piece_size
    }

    /// Get total number of pieces
    pub fn total_pieces(&self) -> u32 {
        self.total_pieces
    }
}
