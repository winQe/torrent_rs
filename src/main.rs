use std::env;
use std::path::PathBuf;

use torrent_rs::client::{ClientConfig, TorrentSession};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    let torrent_path = args.get(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("example/debian-12.7.0-amd64-netinst.iso.torrent")
    });

    let download_path = args.get(2).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("./downloads")
    });

    let config = ClientConfig::default().with_download_path(download_path);

    let session = TorrentSession::new(&torrent_path, config).await?;
    session.start().await?;

    Ok(())
}
