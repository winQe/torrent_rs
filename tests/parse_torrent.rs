use std::path::PathBuf;
use torrent_rs::torrent;

#[test]
fn test_torrent_file_parsing() {
    let torrent_path = PathBuf::from("example/debian-12.7.0-amd64-netinst.iso.torrent");
    let torrent_result = torrent::open(torrent_path.to_string_lossy().to_string());

    assert!(
        torrent_result.is_ok(),
        "Torrent file should parse successfully"
    );

    let torrent = torrent_result.unwrap();

    assert_eq!(torrent.info.name, "debian-12.7.0-amd64-netinst.iso");
    assert!(torrent.info.pieces.len() > 0, "Torrent should have pieces");
    assert!(
        torrent.info.length.unwrap() > 0,
        "Torrent should have a valid total length"
    );
}

#[test]
fn test_invalid_torrent_file_parsing() {
    let invalid_path = PathBuf::from("non_existent_torrent_file.torrent");

    let torrent_result = torrent::open(invalid_path.to_string_lossy().to_string());

    assert!(
        torrent_result.is_err(),
        "Non-existent torrent file should return an error"
    );
}
