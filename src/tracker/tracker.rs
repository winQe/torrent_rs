use std::net::SocketAddrV4;

use anyhow::Context;
use rand::{thread_rng, Rng};
use serde_derive::{Deserialize, Serialize};

use crate::torrent::bencode::Torrent;

const PEER_ID_LENGTH: usize = 20;
type Peer = SocketAddrV4;

#[derive(Debug, Clone, Deserialize)]
pub struct TrackerResponse {
    /// An integer, indicating how often your client should make a request to the tracker in seconds.
    ///
    /// You can ignore this value for the purposes of this challenge.
    pub interval: usize,

    /// A string, which contains list of peers that your client can connect to.
    ///
    /// Each peer is represented using 6 bytes. The first 4 bytes are the peer's IP address and the
    /// last 2 bytes are the peer's port number.
    pub peers: Vec<std::net::SocketAddrV4>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackerRequest {
    /// A unique identifier for your client.
    ///
    /// A string of length 20 that you get to pick.
    pub peer_id: [u8; PEER_ID_LENGTH],

    /// The port your client is listening on.
    /// Typically BitTorrent uses TCP port 6881-6889
    pub port: u16,

    /// The total amount uploaded so far.
    pub uploaded: usize,

    /// The total amount downloaded so far
    pub downloaded: usize,

    /// The number of bytes left to download.
    pub left: usize,

    /// Whether the peer list should use the compact representation
    ///
    /// The compact representation is more commonly used in the wild, the non-compact
    /// representation is mostly supported for backward-compatibility.
    pub compact: u8,
}

impl TrackerRequest {
    fn build_request(torrent: &Torrent) -> anyhow::Result<Self> {
        Ok(TrackerRequest {
            peer_id: Self::generate_peer_id(),
            port: 6889,
            uploaded: 0,
            downloaded: 0,
            left: torrent.info.length.unwrap() as usize,
            compact: 1,
        })
    }

    pub async fn announce(torrent: &Torrent) -> anyhow::Result<TrackerResponse> {
        let request = Self::build_request(torrent)?;
        let params = serde_urlencoded::to_string(&request)
            .context("Failed to encode tracker url params!")?;

        let tracker_url = format!(
            "{}?{}&info_hash={}",
            torrent.announce.clone().unwrap(),
            params,
            torrent.info.root_hash.clone().unwrap(),
        );

        let response = reqwest::get(tracker_url)
            .await
            .context("Failed to make GET request to tracker server!")?;
        let response = response
            .bytes()
            .await
            .context("Failed converting tracker response into bytes!")?;

        let response: TrackerResponse = serde_bencode::from_bytes(&response)
            .context("Failed to deserialize tracker response!")?;

        Ok(response)
    }

    fn generate_peer_id() -> [u8; 20] {
        let mut id = [0u8; PEER_ID_LENGTH];
        let prefix = b"-TR0001-";

        id[..prefix.len()].copy_from_slice(prefix);

        let mut rng = thread_rng();
        for byte in id[prefix.len()..].iter_mut() {
            // Generate random printable ASCII characters (numbers and uppercase letters)
            *byte = rng.gen_range(b'0'..=b'Z');
        }

        id
    }
}
