use super::Peer;
use anyhow::{bail, Context, Ok};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
    time::Duration,
};
use tracing::instrument;

const PROTOCOL_IDENTIFIER_LENGTH: u8 = 19;
const PROTOCOL_IDENTIFIER: [u8; 19] = *b"BitTorrent protocol";
const HANDSHAKE_MESSAGE_LENGTH: usize = 68;

#[derive(Copy, Clone)]
struct HandshakeMessage {
    length: u8,
    pstr: [u8; PROTOCOL_IDENTIFIER_LENGTH as usize],
    reserved: [u8; 8],
    info_hash: [u8; 20],
    peer_id: [u8; 20],
}

impl HandshakeMessage {
    fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(std::mem::size_of::<Self>());
        bytes.push(self.length);
        bytes.extend_from_slice(&self.pstr);
        bytes.extend_from_slice(&self.reserved);
        bytes.extend_from_slice(&self.info_hash);
        bytes.extend_from_slice(&self.peer_id);
        bytes
    }
}

impl Peer {
    //TODO: retry mechanism with exponential backoff
    #[instrument(skip(self))]
    pub async fn handshake(&self) -> anyhow::Result<tokio::net::TcpStream> {
        if self.peer_id.as_bytes().len() != 20 {
            bail!("Peer ID must be exactly 20 bytes long");
        }

        let mut tcp_stream = timeout(
            Duration::from_secs(5),
            tokio::net::TcpStream::connect(self.addr),
        )
        .await
        .context("Establishing TCP stream timed out after 5s")?
        .context("Failed to connect to TCP stream")?;

        let mut info_hash = [0u8; 20];
        info_hash.copy_from_slice(&self.info_hash);

        let mut peer_id = [0u8; 20];
        peer_id.copy_from_slice(self.peer_id.as_bytes());

        let handshake_message = HandshakeMessage {
            length: PROTOCOL_IDENTIFIER_LENGTH,
            pstr: PROTOCOL_IDENTIFIER,
            reserved: [0; 8],
            info_hash,
            peer_id,
        };

        tcp_stream
            .write_all(&handshake_message.to_bytes())
            .await
            .context("Failed to send handshake message!")?;

        // Read the response
        let mut response = vec![0u8; HANDSHAKE_MESSAGE_LENGTH];
        timeout(Duration::from_secs(5), tcp_stream.read_exact(&mut response))
            .await
            .context("Handshake response timed out after 5s")?
            .context("Failed to read handshake response")?;

        // TODO: Should read the external_peer_id here from 48..68
        // Validate the response
        if response[1..20] != PROTOCOL_IDENTIFIER {
            bail!("Invalid protocol identifier in handshake response");
        }

        if response[28..48] != info_hash {
            bail!("Info hash mismatch in handshake response");
        }

        tracing::info!("Handshake with peer {} sucessful", self.addr);
        Ok(tcp_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handshake_message_serialization() {
        let message = HandshakeMessage {
            length: PROTOCOL_IDENTIFIER_LENGTH,
            pstr: PROTOCOL_IDENTIFIER,
            reserved: [0; 8],
            info_hash: [1; 20],
            peer_id: [2; 20],
        };

        let bytes = message.to_bytes();
        assert_eq!(bytes.len(), HANDSHAKE_MESSAGE_LENGTH);
        assert_eq!(bytes[0], PROTOCOL_IDENTIFIER_LENGTH);
        assert_eq!(&bytes[1..20], PROTOCOL_IDENTIFIER);
        assert_eq!(&bytes[20..28], &[0; 8]);
        assert_eq!(&bytes[28..48], &[1; 20]);
        assert_eq!(&bytes[48..68], &[2; 20]);
    }
}
