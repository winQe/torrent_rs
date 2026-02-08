use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

use torrent_rs::client::{ClientConfig, TorrentSession};

/// A BitTorrent client written in Rust
#[derive(Parser)]
#[command(name = "torrent_rs", version)]
struct Args {
    /// Path to the .torrent file
    torrent_file: PathBuf,

    /// Download directory
    #[arg(short, long, default_value = "./downloads")]
    output: PathBuf,

    /// Maximum number of peer connections
    #[arg(short, long, default_value_t = 50)]
    peers: usize,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let filter = if args.verbose {
        EnvFilter::new("torrent_rs=debug")
    } else {
        EnvFilter::new("torrent_rs=error")
    };

    fmt().with_env_filter(filter).init();

    let config = ClientConfig::default()
        .with_download_path(args.output)
        .with_max_peers(args.peers);

    let session = TorrentSession::new(&args.torrent_file, config).await?;
    session.start().await?;

    Ok(())
}
