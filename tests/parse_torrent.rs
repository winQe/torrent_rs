use anyhow::Ok;
use std::path::PathBuf;
use torrent_rs::torrent::Torrent;

#[tokio::test]
async fn test_torrent_file_parsing() -> anyhow::Result<()> {
    let torrent_path = PathBuf::from("example/debian-12.7.0-amd64-netinst.iso.torrent");
    let torrent_result = Torrent::open(torrent_path).await;

    assert!(
        torrent_result.is_ok(),
        "Torrent file should parse successfully"
    );

    let torrent = torrent_result.unwrap();

    assert_eq!(torrent.info.name, "debian-12.7.0-amd64-netinst.iso");
    assert!(
        !torrent.info.pieces.0.is_empty(),
        "Torrent should have pieces"
    );
    assert!(
        torrent.length() > 0,
        "Torrent should have a valid total length"
    );

    assert!(torrent.info_hash.is_some());
    assert_eq!(
        torrent.urlencode_infohash(),
        Some("%1B%D0%88%EE%91%66%A0%62%CF%4A%F0%9C%F9%97%20%FA%6E%1A%31%33".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn test_invalid_torrent_file_parsing() {
    let invalid_path = PathBuf::from("non_existent_torrent_file.torrent");

    let torrent_result = Torrent::open(invalid_path.to_string_lossy().to_string()).await;

    assert!(
        torrent_result.is_err(),
        "Non-existent torrent file should return an error"
    );
}
