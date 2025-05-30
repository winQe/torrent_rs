use std::path::PathBuf;
use torrent_rs::{
    peer::Peer,
    torrent::Torrent,
    tracker::{self, TrackerRequest},
};

#[ignore]
#[tokio::test]
async fn test_peer_handshake() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let torrent_path = PathBuf::from("example/debian-12.7.0-amd64-netinst.iso.torrent");
    let torrent = Torrent::open(torrent_path).await.unwrap();

    let tracker_reponse = tracker::TrackerRequest::announce(&torrent).await;
    assert!(tracker_reponse.is_ok(), "Tracker announce should succeed");

    let response = tracker_reponse.unwrap();

    assert!(response.interval > 0, "Interval should be positive");
    assert!(
        !response.peer_addresses.0.is_empty(),
        "Should receive at least one peer"
    );

    let peer_id = TrackerRequest::generate_peer_id();
    let info_hash = torrent.info_hash.unwrap();

    let mut successful_handshakes = false;

    for &address in response.peer_addresses.iter() {
        let peer = Peer::new(address, info_hash, peer_id.clone());
        let res = peer.handshake().await;
        if res.is_ok() {
            successful_handshakes = true;
            break;
        } else {
            tracing::error!("Peer {:?} failed to handshake", address);
            tracing::error!("{}", res.unwrap_err());
        }
    }

    assert!(
        successful_handshakes,
        "At least one peer handshake should succeed"
    );
    anyhow::Ok(())
}
