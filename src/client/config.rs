use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Directory where downloaded files will be saved
    pub download_path: PathBuf,
    /// Port to listen on for incoming peer connections
    pub listen_port: u16,
    /// Maximum number of peer connections
    pub max_peers: usize,
    /// Number of pipelined block requests per peer (improves throughput)
    pub max_requests_per_peer: usize,
    /// Timeout for establishing peer connections
    pub connection_timeout: Duration,
    /// Timeout for block requests before re-requesting
    pub request_timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            download_path: PathBuf::from("."),
            listen_port: 6881,
            max_peers: 50,
            max_requests_per_peer: 5,
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
        }
    }
}

impl ClientConfig {
    pub fn with_download_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.download_path = path.into();
        self
    }

    pub fn with_max_peers(mut self, max: usize) -> Self {
        self.max_peers = max;
        self
    }

    pub fn with_listen_port(mut self, port: u16) -> Self {
        self.listen_port = port;
        self
    }
}
