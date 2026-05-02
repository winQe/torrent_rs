use std::io;

use tokio_util::bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use super::PeerMessage;

// DDoS Protection - max piece size is typically 256KB-1MB, allow up to 2MB
const MAX_MESSAGE_SIZE: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub struct MessageCodec;

impl Decoder for MessageCodec {
    type Item = PeerMessage;

    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            // Length prefix is 4 bytes
            return Ok(None);
        }

        // Peek at length without consuming
        let length = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        if length == 0 {
            src.advance(4); // Now consume the length prefix
            return Ok(Some(PeerMessage::KeepAlive));
        }

        // DDoS Protection
        if length > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Message length {} exceeds maximum allowed size", length),
            ));
        }

        // Check if we have the full message (4 byte prefix + length bytes)
        if src.len() < 4 + length {
            return Ok(None);
        }

        // Now consume the length prefix
        src.advance(4);

        // ID is a single decimal byte
        let id = src.get_u8();

        let body_len = length - 1;

        let message = match id {
            0 => {
                require_body_len(id, body_len, 0)?;
                PeerMessage::Choke
            }
            1 => {
                require_body_len(id, body_len, 0)?;
                PeerMessage::Unchoke
            }
            2 => {
                require_body_len(id, body_len, 0)?;
                PeerMessage::Interested
            }
            3 => {
                require_body_len(id, body_len, 0)?;
                PeerMessage::NotInterested
            }
            4 => {
                require_body_len(id, body_len, 4)?;
                PeerMessage::Have(src.get_u32())
            }
            5 => {
                let bitfield = src.split_to(body_len).to_vec();
                PeerMessage::Bitfield(bitfield)
            }
            6 => {
                require_body_len(id, body_len, 12)?;
                PeerMessage::Request {
                    index: src.get_u32(),
                    begin: src.get_u32(),
                    length: src.get_u32(),
                }
            }
            7 => {
                if body_len < 8 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Piece message body too short: {body_len}"),
                    ));
                }
                let index = src.get_u32();
                let begin = src.get_u32();
                let block = src.split_to(body_len - 8).to_vec();
                PeerMessage::Piece {
                    index,
                    begin,
                    block,
                }
            }
            8 => {
                require_body_len(id, body_len, 12)?;
                PeerMessage::Cancel {
                    index: src.get_u32(),
                    begin: src.get_u32(),
                    length: src.get_u32(),
                }
            }
            9 => {
                require_body_len(id, body_len, 2)?;
                PeerMessage::Port(src.get_u16())
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

fn require_body_len(id: u8, actual: usize, expected: usize) -> Result<(), io::Error> {
    if actual != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Message id {id} expects {expected} body bytes, got {actual}"),
        ));
    }
    Ok(())
}

impl Encoder<PeerMessage> for MessageCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: PeerMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            PeerMessage::KeepAlive => {
                dst.put_u32(0); // Length prefix is 0 for KeepAlive
            }
            PeerMessage::Choke => {
                dst.put_u32(1);
                dst.put_u8(0);
            }
            PeerMessage::Unchoke => {
                dst.put_u32(1);
                dst.put_u8(1);
            }
            PeerMessage::Interested => {
                dst.put_u32(1);
                dst.put_u8(2);
            }
            PeerMessage::NotInterested => {
                dst.put_u32(1);
                dst.put_u8(3);
            }
            PeerMessage::Have(index) => {
                dst.put_u32(5); // Length prefix
                dst.put_u8(4); // Message ID
                dst.put_u32(index);
            }
            PeerMessage::Bitfield(bitfield) => {
                dst.put_u32(1 + bitfield.len() as u32);
                dst.put_u8(5);
                dst.extend_from_slice(&bitfield);
            }
            PeerMessage::Request {
                index,
                begin,
                length,
            } => {
                dst.put_u32(13); // Length prefix
                dst.put_u8(6); // Message ID
                dst.put_u32(index);
                dst.put_u32(begin);
                dst.put_u32(length);
            }
            PeerMessage::Piece {
                index,
                begin,
                block,
            } => {
                dst.put_u32(9 + block.len() as u32); // Length prefix
                dst.put_u8(7); // Message ID
                dst.put_u32(index);
                dst.put_u32(begin);
                dst.extend_from_slice(&block);
            }
            PeerMessage::Cancel {
                index,
                begin,
                length,
            } => {
                dst.put_u32(13); // Length prefix
                dst.put_u8(8); // Message ID
                dst.put_u32(index);
                dst.put_u32(begin);
                dst.put_u32(length);
            }
            PeerMessage::Port(port) => {
                dst.put_u32(3);
                dst.put_u8(9);
                dst.put_u16(port);
            }
        }
        Ok(())
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
            assert!(e.to_string().contains("exceeds maximum allowed size"));
        }
    }

    #[test]
    fn test_decode_have_with_short_body_errors() {
        let mut codec = MessageCodec;
        // claims length 2 (id + 1 byte body) for Have, which needs 4-byte body
        let mut buffer = BytesMut::from(&[0, 0, 0, 2, 4, 0xFF][..]);
        let result = codec.decode(&mut buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_request_with_short_body_errors() {
        let mut codec = MessageCodec;
        // claims length 5 (id + 4 byte body) for Request, which needs 12-byte body
        let mut buffer = BytesMut::from(&[0, 0, 0, 5, 6, 0, 0, 0, 1][..]);
        let result = codec.decode(&mut buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_piece_with_short_body_errors() {
        let mut codec = MessageCodec;
        // claims length 5 (4 body bytes) for Piece, which needs at least 8
        let mut buffer = BytesMut::from(&[0, 0, 0, 5, 7, 0, 0, 0, 1][..]);
        let result = codec.decode(&mut buffer);
        assert!(result.is_err());
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
