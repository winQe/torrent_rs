use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::task::JoinSet;
use tracing::{error, info, warn};

use crate::file::{DiskFileManager, FileManager};
use crate::piece::verify_piece;
use crate::torrent::{Keys, Torrent};
use crate::tracker::TrackerRequest;

use super::config::ClientConfig;
use super::peer_worker::PeerWorker;
use super::state::{CompletedPiece, SharedState};

/// Main session coordinator for downloading a torrent.
pub struct TorrentSession {
    torrent: Torrent,
    config: ClientConfig,
    peer_id: String,
}

impl TorrentSession {
    /// Create a new session for downloading a torrent file.
    pub async fn new(
        torrent_path: impl AsRef<Path> + std::fmt::Debug,
        config: ClientConfig,
    ) -> Result<Self> {
        let torrent = Torrent::open(torrent_path)
            .await
            .context("Failed to open torrent file")?;

        let peer_id = TrackerRequest::generate_peer_id();

        Ok(Self {
            torrent,
            config,
            peer_id,
        })
    }

    /// Start downloading the torrent.
    pub async fn start(self) -> Result<()> {
        let total_length = self.torrent.length() as u64;
        let piece_size = self.torrent.info.piece_length as u32;
        let total_pieces = self.torrent.info.pieces.0.len() as u32;
        let info_hash = self
            .torrent
            .info_hash
            .context("Torrent missing info hash")?;

        // Initialize shared state
        let state = SharedState::new(total_pieces, piece_size);

        // Set up channels
        let (piece_tx, piece_rx) = mpsc::channel::<CompletedPiece>(100);
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        // Set up disk file manager
        let files = self.get_file_info();
        let disk_manager =
            DiskFileManager::new(self.config.download_path.clone(), files, piece_size)
                .context("Failed to create disk manager")?;
        let disk_manager = Arc::new(tokio::sync::Mutex::new(disk_manager));

        // Spawn piece writer/verifier task
        let writer_state = Arc::clone(&state);
        let writer_disk = Arc::clone(&disk_manager);
        let piece_hashes = self.torrent.info.pieces.0.clone();
        let writer_shutdown = shutdown_tx.subscribe();

        let writer_handle = tokio::spawn(async move {
            piece_writer_task(
                piece_rx,
                piece_hashes,
                writer_state,
                writer_disk,
                writer_shutdown,
            )
            .await
        });

        // Announce to tracker and get peers
        let tracker_response = TrackerRequest::announce(&self.torrent)
            .await
            .context("Failed to announce to tracker")?;

        let peer_count = tracker_response.peer_addresses.0.len();

        // Print startup banner
        println!("Torrent: {}", self.torrent.info.name);
        println!(
            "Size:    {} ({} pieces)",
            format_bytes(total_length),
            total_pieces
        );
        println!("Tracker: {}", self.torrent.announce);
        println!("Peers:   {} found", peer_count);
        println!();

        if peer_count == 0 {
            eprintln!("No peers available from tracker");
            return Ok(());
        }

        // Set up progress bar
        let pb = ProgressBar::new(total_pieces as u64);
        pb.set_style(
            ProgressStyle::with_template("{bar:40.cyan/blue} {pos}/{len} pieces  {msg}")
                .unwrap()
                .progress_chars("##-"),
        );

        // Spawn peer workers with concurrency limit
        let semaphore = Arc::new(Semaphore::new(self.config.max_peers));
        let mut peer_handles = JoinSet::new();

        for addr in tracker_response.peer_addresses.iter() {
            let permit = semaphore.clone().acquire_owned().await?;

            let worker = PeerWorker::new(
                *addr,
                info_hash,
                self.peer_id.clone(),
                Arc::clone(&state),
                self.config.clone(),
                piece_tx.clone(),
                shutdown_tx.subscribe(),
                total_length,
                piece_size,
                total_pieces,
            );

            peer_handles.spawn(async move {
                let result = worker.run().await;
                drop(permit);
                result
            });
        }

        // Drop our sender so writer task can detect completion
        drop(piece_tx);

        // Progress update task
        let progress_state = Arc::clone(&state);
        let progress_pb = pb.clone();
        let progress_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                let stats = &progress_state.stats;
                let completed = stats.pieces_completed();
                let downloaded = stats.downloaded_bytes();
                let speed = stats.download_speed();

                progress_pb.set_position(completed);
                progress_pb.set_message(format!(
                    "{}  {}/s",
                    format_bytes(downloaded),
                    format_bytes(speed as u64),
                ));

                if completed as u32 == stats.total_pieces() {
                    break;
                }
            }
        });

        // Wait for completion, all peers to disconnect, or Ctrl+C
        loop {
            tokio::select! {
                result = peer_handles.join_next() => {
                    match result {
                        Some(Err(e)) => warn!("Peer task panicked: {}", e),
                        Some(Ok(_)) => {}
                        None => break, // all peers done
                    }

                    // Check if download is complete
                    let pm = state.piece_manager.read().await;
                    if pm.is_complete() {
                        break;
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    pb.finish_and_clear();
                    eprintln!("\nShutting down...");
                    let _ = shutdown_tx.send(());
                    peer_handles.abort_all();
                    let _ = writer_handle.await;
                    progress_handle.abort();

                    let stats = &state.stats;
                    eprintln!(
                        "Downloaded {}/{} pieces ({})",
                        stats.pieces_completed(),
                        stats.total_pieces(),
                        format_bytes(stats.downloaded_bytes()),
                    );
                    return Ok(());
                }
            }
        }

        // Signal shutdown
        let _ = shutdown_tx.send(());

        // Wait for writer task
        let _ = writer_handle.await;

        // Cancel progress task
        progress_handle.abort();

        let stats = &state.stats;
        pb.finish_with_message(format!("{}  done!", format_bytes(stats.downloaded_bytes()),));

        println!(
            "\nDownload complete: {}/{} pieces",
            stats.pieces_completed(),
            stats.total_pieces(),
        );

        Ok(())
    }

    /// Extract file information from torrent for disk manager
    fn get_file_info(&self) -> Vec<(String, u64)> {
        match &self.torrent.info.keys {
            Keys::SingleFile { length } => {
                vec![(self.torrent.info.name.clone(), *length as u64)]
            }
            Keys::MultiFile { files } => files
                .iter()
                .map(|f| {
                    let path = f.path.join(std::path::MAIN_SEPARATOR_STR);
                    (path, f.length as u64)
                })
                .collect(),
        }
    }
}

/// Format byte count as human-readable string (e.g. "631.0 MB").
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Background task that verifies and writes completed pieces to disk.
async fn piece_writer_task(
    mut rx: mpsc::Receiver<CompletedPiece>,
    piece_hashes: Vec<[u8; 20]>,
    state: Arc<SharedState>,
    disk: Arc<tokio::sync::Mutex<DiskFileManager>>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    loop {
        tokio::select! {
            biased;

            _ = shutdown_rx.recv() => {
                break;
            }

            piece = rx.recv() => {
                match piece {
                    Some(completed) => {
                        let index = completed.index as usize;

                        // Verify hash
                        if index >= piece_hashes.len() {
                            error!("Piece {} index out of bounds", completed.index);
                            continue;
                        }

                        let expected_hash = &piece_hashes[index];
                        if !verify_piece(&completed.data, expected_hash) {
                            warn!("Piece {} failed hash verification, re-queuing", completed.index);
                            let mut pm = state.piece_manager.write().await;
                            pm.mark_failed(completed.index);
                            continue;
                        }

                        // Write to disk
                        {
                            let mut disk = disk.lock().await;
                            if let Err(e) = disk.write_piece(completed.index, &completed.data) {
                                error!("Failed to write piece {}: {}", completed.index, e);
                                let mut pm = state.piece_manager.write().await;
                                pm.mark_failed(completed.index);
                                continue;
                            }
                        }

                        // Mark as completed
                        {
                            let mut pm = state.piece_manager.write().await;
                            pm.mark_completed(completed.index);
                        }
                        {
                            let mut completed_set = state.completed_pieces.write().await;
                            completed_set.insert(completed.index);
                        }
                        state.stats.increment_pieces();

                        info!("Piece {} verified and written to disk", completed.index);
                    }
                    None => {
                        // Channel closed, all senders dropped
                        break;
                    }
                }
            }
        }
    }
}
