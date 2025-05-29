use anyhow::{bail, Context};

use super::Peer;
use crate::{
    message::{Bitfield, MessageCodec, PeerMessage},
    piece::BlockInfo,
};
use futures::{SinkExt, StreamExt};

impl Peer {
    pub async fn receive_bitfield(&mut self) -> anyhow::Result<&Bitfield> {
        let tcp_stream = self.handshake().await.context("Failed to handshake")?;
        let mut frame = tokio_util::codec::Framed::new(tcp_stream, MessageCodec);

        let bitfield = frame
            .next()
            .await
            .context("Failed to get the next TCP frame")?
            .context("Failed to receive bitfield")?;

        match bitfield {
            PeerMessage::Bitfield(data) => {
                self.bitfield = Some(Bitfield::from_bytes(data));
            }
            _ => {
                bail!("First message is not bitfield");
            }
        }

        self.tcp_stream = Some(frame);

        self.bitfield()
            .context("Bitfield was not set after successful connection")
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
}
