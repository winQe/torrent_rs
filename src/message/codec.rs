use std::io;

use tokio_util::bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

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

        // DDoS Protection
        if length > MAX_MESSAGE_SIZE || src.len() > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Message length exceeds maximum allowed size",
            ));
        }

        // Not full frame is  received, wait for more
        if src.len() < length {
            return Ok(None);
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
            9 => {
                let port = src.get_u16();
                PeerMessage::Port(port)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::bytes::BytesMut;
    use tokio_util::codec::Decoder;

    #[test]
    fn test_decode_keep_alive() {
        let mut codec = MessageCodec;
        let mut buffer = BytesMut::from(&[0, 0, 0, 0][..]); // KeepAlive message
        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(message, Some(PeerMessage::KeepAlive));
    }

    #[test]
    fn test_decode_choke() {
        let mut codec = MessageCodec;
        let mut buffer = BytesMut::from(&[0, 0, 0, 1, 0][..]); // Choke message
        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(message, Some(PeerMessage::Choke));
    }

    #[test]
    fn test_decode_have() {
        let mut codec = MessageCodec;
        let mut buffer = BytesMut::from(&[0, 0, 0, 5, 4, 0, 0, 0, 42][..]); // Have(42)
        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(message, Some(PeerMessage::Have(42)));
    }

    #[test]
    fn test_incomplete_buffer() {
        let mut codec = MessageCodec;
        let mut buffer = BytesMut::from(&[0, 0, 0, 5, 4, 0, 0][..]); // Incomplete "Have"
        let message = codec.decode(&mut buffer).unwrap();
        assert!(message.is_none());
    }

    #[test]
    fn test_invalid_message_id() {
        let mut codec = MessageCodec;
        let mut buffer = BytesMut::from(&[0, 0, 0, 1, 99][..]); // Invalid ID 99
        let result = codec.decode(&mut buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_excessive_length() {
        let mut codec = MessageCodec;
        // Create a message length that exceeds MAX_MESSAGE_SIZE
        let excessive_length = (MAX_MESSAGE_SIZE + 1) as u32;
        let mut buffer = BytesMut::new();
        buffer.extend_from_slice(&excessive_length.to_be_bytes());
        buffer.extend_from_slice(&[0]);

        let result = codec.decode(&mut buffer);
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.to_string(), "Message length exceeds maximum allowed size");
        }
    }

    #[test]
    fn test_decode_bitfield() {
        let mut codec = MessageCodec;
        let mut buffer = BytesMut::from(&[0, 0, 0, 3, 5, 0b10101010, 0b11110000][..]);
        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(
            message,
            Some(PeerMessage::Bitfield(vec![0b10101010, 0b11110000]))
        );
    }
}
