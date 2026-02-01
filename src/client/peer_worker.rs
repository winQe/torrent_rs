use std::collections::VecDeque;
use std::net::SocketAddrV4;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

use crate::message::{PeerMessage, PieceIndex};
use crate::peer::Peer;
use crate::piece::BlockInfo;

use super::config::ClientConfig;
use super::state::{CompletedPiece, SharedState};

/// Handles communication with a single peer.
/// Each peer connection runs as its own async task.
pub struct PeerWorker {
    peer: Peer,
    state: Arc<SharedState>,
    config: ClientConfig,
    /// Channel to send completed pieces for verification
    piece_tx: mpsc::Sender<CompletedPiece>,
    /// Receives shutdown signal
    shutdown_rx: broadcast::Receiver<()>,
    /// Currently assigned piece (if any)
    assigned_piece: Option<PieceIndex>,
    /// Outstanding block requests (for pipelining)
    pending_requests: VecDeque<BlockInfo>,
    /// Total length of the torrent (for calculating last piece size)
    total_length: u64,
    /// Piece size from torrent
    piece_size: u32,
    /// Total number of pieces
    total_pieces: u32,
}

impl PeerWorker {
    pub fn new(
        addr: SocketAddrV4,
        info_hash: [u8; 20],
        peer_id: String,
        state: Arc<SharedState>,
        config: ClientConfig,
        piece_tx: mpsc::Sender<CompletedPiece>,
        shutdown_rx: broadcast::Receiver<()>,
        total_length: u64,
        piece_size: u32,
        total_pieces: u32,
    ) -> Self {
        Self {
            peer: Peer::new(addr, info_hash, peer_id),
            state,
            config,
            piece_tx,
            shutdown_rx,
            assigned_piece: None,
            pending_requests: VecDeque::new(),
            total_length,
            piece_size,
            total_pieces,
        }
    }

    /// Main worker loop - connects, handshakes, and processes messages.
    pub async fn run(mut self) -> Result<()> {
        let addr = self.peer.address();
        debug!("Connecting to peer {}", addr);

        // Connect and receive bitfield
        let bitfield = self
            .peer
            .receive_bitfield()
            .await
            .with_context(|| format!("Failed to connect to peer {}", addr))?;

        info!("Connected to peer {}, received bitfield", addr);

        // Update piece availability in shared state
        {
            let mut pm = self.state.piece_manager.write().await;
            pm.add_peer(bitfield);
        }

        // Express interest in downloading
        self.peer.send_interested().await?;
        self.peer.set_interested(true);

        // Main message loop
        loop {
            tokio::select! {
                biased;

                // Check for shutdown signal
                _ = self.shutdown_rx.recv() => {
                    debug!("Peer {} received shutdown signal", addr);
                    break;
                }

                // Receive and handle messages
                msg = self.peer.receive_message() => {
                    match msg {
                        Ok(Some(message)) => {
                            if let Err(e) = self.handle_message(message).await {
                                warn!("Error handling message from {}: {}", addr, e);
                                break;
                            }
                        }
                        Ok(None) => {
                            debug!("Peer {} disconnected", addr);
                            break;
                        }
                        Err(e) => {
                            warn!("Error receiving from {}: {}", addr, e);
                            break;
                        }
                    }
                }
            }
        }

        // Cleanup: remove peer's contribution to availability
        if let Some(bitfield) = self.peer.bitfield() {
            let mut pm = self.state.piece_manager.write().await;
            pm.remove_peer(bitfield);
        }

        // Return any assigned piece to the pool
        if let Some(piece) = self.assigned_piece.take() {
            let mut pm = self.state.piece_manager.write().await;
            pm.mark_failed(piece);
        }

        Ok(())
    }

    async fn handle_message(&mut self, message: PeerMessage) -> Result<()> {
        match message {
            PeerMessage::Choke => {
                debug!("Peer {} choked us", self.peer.address());
                self.peer.choke();
                // Clear pending requests - they won't be fulfilled
                self.pending_requests.clear();
            }

            PeerMessage::Unchoke => {
                debug!("Peer {} unchoked us", self.peer.address());
                self.peer.unchoke();
                // Start requesting blocks
                self.request_more_blocks().await?;
            }

            PeerMessage::Have(piece_index) => {
                // Peer got a new piece, update availability
                // For simplicity, we don't update the BTreeSet here
                // as it would require the full bitfield
                debug!("Peer {} has piece {}", self.peer.address(), piece_index);
            }

            PeerMessage::Piece { index, begin, block } => {
                self.handle_piece_data(index, begin, block).await?;
            }

            PeerMessage::KeepAlive => {
                // Nothing to do, connection is still alive
            }

            PeerMessage::Interested | PeerMessage::NotInterested => {
                // We're not uploading yet, ignore these
            }

            PeerMessage::Request { .. } | PeerMessage::Cancel { .. } => {
                // Upload requests - not implemented yet
            }

            PeerMessage::Bitfield(_) | PeerMessage::Port(_) => {
                // Unexpected at this point
            }
        }

        Ok(())
    }

    async fn handle_piece_data(&mut self, index: PieceIndex, begin: u32, block: Vec<u8>) -> Result<()> {
        let block_info = BlockInfo {
            piece_index: index,
            offset: begin,
            length: block.len() as u32,
        };

        // Remove from pending requests
        self.pending_requests.retain(|b| {
            !(b.piece_index == index && b.offset == begin)
        });

        // Store the block
        {
            let mut bm = self.state.block_manager.lock().await;
            bm.store_block(block_info, block.clone());

            // Check if piece is complete
            if bm.is_piece_complete(index) {
                if let Some(data) = bm.assemble_piece(index) {
                    debug!("Piece {} complete, sending for verification", index);

                    // Send to verification/writer task
                    let completed = CompletedPiece { index, data };
                    if self.piece_tx.send(completed).await.is_err() {
                        // Channel closed, session is shutting down
                        return Ok(());
                    }

                    // Cleanup block manager
                    bm.cleanup_piece(index);

                    // Clear assignment so we can get a new piece
                    self.assigned_piece = None;
                }
            }
        }

        // Update download stats
        self.state.stats.add_downloaded(block.len() as u64);

        // Request more blocks to keep pipeline full
        if !self.peer.is_choked() {
            self.request_more_blocks().await?;
        }

        Ok(())
    }

    async fn request_more_blocks(&mut self) -> Result<()> {
        // Fill pipeline with requests
        while self.pending_requests.len() < self.config.max_requests_per_peer {
            // Get or assign a piece to work on
            if self.assigned_piece.is_none() {
                let mut pm = self.state.piece_manager.write().await;
                if let Some(piece) = pm.next_piece() {
                    self.assigned_piece = Some(piece);

                    // Initialize piece in block manager
                    let piece_size = self.get_piece_size(piece);
                    let mut bm = self.state.block_manager.lock().await;
                    bm.init_piece(piece, piece_size);

                    debug!("Assigned piece {} to peer {}", piece, self.peer.address());
                }
            }

            // Request next block from assigned piece
            if let Some(piece) = self.assigned_piece {
                let piece_size = self.get_piece_size(piece);
                let mut bm = self.state.block_manager.lock().await;

                if let Some(block_info) = bm.next_block(piece, piece_size) {
                    drop(bm); // Release lock before async operation

                    self.peer.request_block(block_info).await?;
                    self.pending_requests.push_back(block_info);
                } else {
                    // No more blocks to request for this piece
                    // Either all requested or all received
                    break;
                }
            } else {
                // No pieces available to download
                break;
            }
        }

        Ok(())
    }

    /// Calculate the size of a specific piece (last piece may be smaller)
    fn get_piece_size(&self, piece_index: PieceIndex) -> u32 {
        if piece_index == self.total_pieces - 1 {
            // Last piece
            let remainder = self.total_length % self.piece_size as u64;
            if remainder == 0 {
                self.piece_size
            } else {
                remainder as u32
            }
        } else {
            self.piece_size
        }
    }
}
