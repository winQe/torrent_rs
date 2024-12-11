use anyhow::Context;
use rand::Rng;
use serde_derive::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::peer::PeerAddresses;
use crate::torrent::Torrent;

#[derive(Debug, Clone, Deserialize)]
pub struct TrackerResponse {
    /// An integer, indicating how often your client should make a request to the tracker in seconds.
    pub interval: usize,

    /// A string, which contains list of peers that your client can connect to.
    ///
    /// Each peer is represented using 6 bytes. The first 4 bytes are the peer's IP address and the
    /// last 2 bytes are the peer's port number.
    #[serde(rename = "peers")]
    pub peer_addresses: PeerAddresses,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackerRequest {
    /// A unique identifier for your client.
    ///
    /// A string of length 20 that you get to pick.
    pub peer_id: String,

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
            left: torrent.length(),
            compact: 1,
        })
    }
    #[instrument(skip(torrent))]
    pub async fn announce(torrent: &Torrent) -> anyhow::Result<TrackerResponse> {
        let request = Self::build_request(torrent).context("Failed to build request")?;
        let params = serde_urlencoded::to_string(&request)
            .context("Failed to encode tracker url params!")?;
        let info_hash_urlencoded = torrent
            .urlencode_infohash()
            .context("Failed to urlencode infohash")?;

        let tracker_url = format!(
            "{}?{}&info_hash={}",
            torrent.announce, params, info_hash_urlencoded,
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

        info!("Sucesfully retrieved peers from tracker");

        Ok(response)
    }

    pub fn generate_peer_id() -> String {
        let mut rng = rand::thread_rng();
        let prefix = "-TR0001-";
        let mut peer_id = String::with_capacity(20);
        peer_id.push_str(prefix);

        // Fill the rest with alphanumeric characters
        for _ in prefix.len()..20 {
            let char = match rng.gen_range(0..3) {
                0 => rng.gen_range(b'A'..=b'Z') as char,
                1 => rng.gen_range(b'a'..=b'z') as char,
                _ => rng.gen_range(b'0'..=b'9') as char,
            };
            peer_id.push(char);
        }

        peer_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Ok, Result};
    use std::net::{Ipv4Addr, SocketAddrV4};
    use tokio;

    #[tokio::test]
    async fn test_announce_success() -> Result<()> {
        use crate::torrent::{Hashes, Info, Keys, Torrent};

        let mut mock_server = mockito::Server::new_async().await;

        let peers = [
            192, 0, 2, 123, 0x1A, 0xE1, // 0x1AE1 = 6881
            127, 0, 0, 1, 0x1A, 0xE9, // 0x1AE9 = 6889
        ];
        let mut response_body = Vec::new();
        response_body.extend_from_slice(b"d8:intervali900e5:peers12:");
        response_body.extend_from_slice(&peers);
        response_body.extend_from_slice(b"e");

        let mock = mock_server
            .mock("GET", "/announce")
            .match_query(mockito::Matcher::Any)
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/x-bencoded")
            .with_body(response_body)
            .create();

        let torrent = Torrent {
            announce: format!("{}/announce", mock_server.url()),
            info: Info {
                name: "mock_torrent".to_string(),
                piece_length: 256 * 1024, // 256 KB
                pieces: Hashes(vec![[0u8; 20]]),
                keys: Keys::SingleFile {
                    length: 1024 * 1024, // 1 MB
                },
            },
            info_hash: Some([0u8; 20]), // Mock 20-byte info hash
        };

        let result = TrackerRequest::announce(&torrent).await;

        assert!(result.is_ok());
        let response = result.unwrap();

        assert_eq!(response.interval, 900);

        let expected_peers = PeerAddresses(vec![
            SocketAddrV4::new(Ipv4Addr::new(192, 0, 2, 123), 6881),
            SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 6889),
        ]);
        assert_eq!(response.peer_addresses, expected_peers);

        mock.assert();
        Ok(())
    }
}
