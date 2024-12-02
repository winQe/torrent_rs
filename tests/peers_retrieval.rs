use anyhow::{Ok, Result};
use std::path::PathBuf;

use torrent_rs::{torrent::Torrent, tracker};

#[tokio::test]
async fn retrieve_peers() -> Result<()> {
    let torrent_path = PathBuf::from("example/debian-12.7.0-amd64-netinst.iso.torrent");
    let torrent = Torrent::open(torrent_path).await?;

    let tracker_reponse = tracker::TrackerRequest::announce(&torrent).await;
    assert!(tracker_reponse.is_ok(), "Tracker announce should succeed");

    let response = tracker_reponse.unwrap();

    assert!(response.interval > 0, "Interval should be positive");
    assert!(
        !response.peer_addresses.0.is_empty(),
        "Should receive at least one peer"
    );

    Ok(())
}
