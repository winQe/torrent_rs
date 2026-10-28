use anyhow::Context;

use super::Peer;
use crate::{
    message::{Bitfield, MessageCodec, PeerMessage},
    piece::BlockInfo,
};
use futures::SinkExt;
use futures::StreamExt;
use std::time::Duration;
use tracing::debug;

impl Peer {
    /// Establish the TCP connection and complete the BitTorrent handshake.
    /// Initializes an empty bitfield sized to the torrent; pieces are added
    /// later via `Bitfield` and `Have` messages from the peer.
    pub async fn connect(&mut self, total_pieces: usize) -> anyhow::Result<()> {
        let tcp_stream = self.handshake().await.context("Failed to handshake")?;
        let frame = tokio_util::codec::Framed::new(tcp_stream, MessageCodec);
        self.tcp_stream = Some(frame);
        self.bitfield = Some(Bitfield::empty(total_pieces));
        Ok(())
    }

    /// Like `connect`, but retries failed attempts with exponential backoff
    /// (`base_delay`, then twice that, and so on) up to `attempts` tries in total.
    pub async fn connect_with_retry(
        &mut self,
        total_pieces: usize,
        attempts: u32,
        base_delay: Duration,
    ) -> anyhow::Result<()> {
        let mut delay = base_delay;
        for attempt in 1..attempts {
            match self.connect(total_pieces).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    debug!(
                        "Connect to {} failed (attempt {}/{}): {:#}, retrying in {:?}",
                        self.addr, attempt, attempts, e, delay
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
            }
        }
        self.connect(total_pieces).await
    }

    pub async fn request_block(&mut self, block_info: BlockInfo) -> anyhow::Result<()> {
        let request_msg = PeerMessage::Request {
            index: block_info.piece_index,
            begin: block_info.offset,
            length: block_info.length,
        };

        self.tcp_stream
            .as_mut()
            .context("TCP stream not initialized")?
            .send(request_msg)
            .await
            .context("Failed to send block request")?;

        Ok(())
    }

    pub async fn send_interested(&mut self) -> anyhow::Result<()> {
        self.tcp_stream
            .as_mut()
            .context("TCP stream not initialized")?
            .send(PeerMessage::Interested)
            .await
            .context("Failed to send interested")?;

        Ok(())
    }

    pub async fn send_not_interested(&mut self) -> anyhow::Result<()> {
        self.tcp_stream
            .as_mut()
            .context("TCP stream not initialized")?
            .send(PeerMessage::NotInterested)
            .await
            .context("Failed to send not interested")?;

        Ok(())
    }

    /// Receive the next message from the peer.
    /// Returns None if the connection is closed.
    pub async fn receive_message(&mut self) -> anyhow::Result<Option<PeerMessage>> {
        let msg = self
            .tcp_stream
            .as_mut()
            .context("TCP stream not initialized")?
            .next()
            .await;

        match msg {
            Some(Ok(message)) => Ok(Some(message)),
            Some(Err(e)) => Err(e).context("Failed to decode message"),
            None => Ok(None), // Connection closed
        }
    }

    /// Send a message to the peer
    pub async fn send_message(&mut self, msg: PeerMessage) -> anyhow::Result<()> {
        self.tcp_stream
            .as_mut()
            .context("TCP stream not initialized")?
            .send(msg)
            .await
            .context("Failed to send message")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::time::Instant;

    #[tokio::test]
    async fn connect_with_retry_backs_off_then_fails() {
        // Bind then drop a listener to get a port that refuses connections.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
        let mut peer = Peer::new(addr, [0; 20], "-TR0001-aaaaaaaaaaaa".to_string());

        let start = Instant::now();
        let result = peer
            .connect_with_retry(1, 3, Duration::from_millis(20))
            .await;

        assert!(result.is_err());
        // Two sleeps between three attempts: 20ms + 40ms.
        assert!(start.elapsed() >= Duration::from_millis(60));
    }
}
