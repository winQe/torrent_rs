use torrent_rs::torrent;
use tracing::info;
use tracing_subscriber;

fn main() {
    tracing_subscriber::fmt::init();

    let torrent =
        torrent::open("example/debian-12.7.0-amd64-netinst.iso.torrent".to_string()).unwrap();
    info!("{:?}", torrent)
}
