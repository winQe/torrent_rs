use anyhow::{bail, Context, Ok};
use futures::StreamExt;

use super::Peer;
use crate::message::{Bitfield, MessageCodec, PeerMessage};

impl Peer {
    pub async fn connect(&mut self) -> anyhow::Result<&Bitfield> {
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
}
