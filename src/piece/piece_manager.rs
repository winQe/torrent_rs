
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
            self.add_piece(piece_index);
        }
    }

    /// Increment availability for a single piece (e.g. on a `Have` announcement).
    pub fn add_piece(&mut self, piece_index: PieceIndex) {
        if self.completed.contains(&piece_index) {
            return;
        }
        let entry = self.piece_counts.entry(piece_index).or_insert(0);
        let old_count = *entry;
        *entry += 1;
        if old_count > 0 {
            self.availability_queue.remove(&(old_count, piece_index));
        }
        self.availability_queue.insert((*entry, piece_index));
    }

    /// Select the next piece this peer can serve, using rarest-first across the swarm.
    /// Returns None if the peer has no piece we still need.
    pub fn next_piece_for(&mut self, peer_bitfield: &Bitfield) -> Option<PieceIndex> {
        let candidate = self
            .availability_queue
            .iter()
            .find(|&&(count, piece)| {
                count > 0
                    && !self.completed.contains(&piece)
                    && !self.pending.contains(&piece)
                    && peer_bitfield.has_piece(piece as usize)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_piece_for_returns_only_what_peer_has() {
        let mut pm = PieceManager::new(3, 16384);
        // Seed availability so all three pieces have count >= 1
        pm.add_peer(&Bitfield::from_bytes(vec![0b1110_0000]));

        // Asking peer only has piece 1
        let asking = Bitfield::from_bytes(vec![0b0100_0000]);
        assert_eq!(pm.next_piece_for(&asking), Some(1));
    }

    #[test]
    fn test_next_piece_for_returns_none_when_peer_has_nothing_useful() {
        let mut pm = PieceManager::new(3, 16384);
        pm.add_peer(&Bitfield::from_bytes(vec![0b1110_0000]));

        let empty = Bitfield::from_bytes(vec![0b0000_0000]);
        assert_eq!(pm.next_piece_for(&empty), None);
    }

    #[test]
    fn test_add_piece_makes_piece_available() {
        let mut pm = PieceManager::new(3, 16384);
        // Nobody has anything yet
        let bf = Bitfield::from_bytes(vec![0b1110_0000]);
        assert_eq!(pm.next_piece_for(&bf), None);

        // Peer announces piece 1 via Have
        pm.add_piece(1);
        assert_eq!(pm.next_piece_for(&bf), Some(1));
    }

    #[test]
    fn test_next_piece_for_skips_pending_pieces_even_if_peer_has_them() {
        let mut pm = PieceManager::new(3, 16384);
        pm.add_peer(&Bitfield::from_bytes(vec![0b1110_0000]));

        let bf = Bitfield::from_bytes(vec![0b1110_0000]);
        let first = pm.next_piece_for(&bf).unwrap();
        let second = pm.next_piece_for(&bf).unwrap();
        assert_ne!(first, second, "concurrent peers must not be assigned the same piece");
    }
}
