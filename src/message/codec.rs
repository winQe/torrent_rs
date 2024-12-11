use std::io;

use tokio_util::bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;

use super::PeerMessage;

// DDoS Protection
const MAX_MESSAGE_SIZE: usize = 16 * 1024; // 16 MB
struct MessageCodec;

impl Decoder for MessageCodec {
    type Item = PeerMessage;

    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            // Length prefix is 4 bytes
            return Ok(None);
        }

        let length = src.get_u32() as usize;
        if length == 0 {
            return Ok(Some(PeerMessage::KeepAlive));
        }

        // Not full frame is  received, wait for more
        if src.len() < length {
            return Ok(None);
        }

        // DDoS Protection
        if length > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Message length exceeds maximum allowed size",
            ));
        }

        // ID is a single decimal byte
        let id = src.get_u8();

        let message = match id {
            0 => PeerMessage::Choke,
            1 => PeerMessage::Unchoke,
            2 => PeerMessage::Interested,
            3 => PeerMessage::NotInterested,
            4 => {
                let piece_index = src.get_u32();
                PeerMessage::Have(piece_index)
            }
            5 => {
                let bitfield = src.split_to(length - 1).to_vec(); // Excluding the ID
                PeerMessage::Bitfield(bitfield)
            }
            6 => {
                let index = src.get_u32();
                let begin = src.get_u32();
                let length = src.get_u32();
                PeerMessage::Request {
                    index,
                    begin,
                    length,
                }
            }
            7 => {
                let index = src.get_u32();
                let begin = src.get_u32();
                // IDs, index and begin are 9 bits
                let block = src.split_to(length - 9).to_vec();
                PeerMessage::Piece {
                    index,
                    begin,
                    block,
                }
            }
            8 => {
                let index = src.get_u32();
                let begin = src.get_u32();
                let length = src.get_u32();
                PeerMessage::Cancel {
                    index,
                    begin,
                    length,
                }
            }

            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Unknown message ID {}", id),
                ))
            }
        };

        Ok(Some(message))
    }
}
