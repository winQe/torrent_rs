// https://github.com/jonhoo/codecrafters-bittorrent-rust/blob/master/src/tracker.rs
use crate::peer::PeerAddresses;

use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};
use std::fmt;
use std::net::{Ipv4Addr, SocketAddrV4};

struct PeerAddressesVisitor;

impl<'de> Visitor<'de> for PeerAddressesVisitor {
    type Value = PeerAddresses;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("6 bytes, the first 4 bytes are a peer's IP address and the last 2 are a peer's port number")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if !v.len().is_multiple_of(6) {
            return Err(E::custom(format!("length is {}", v.len())));
        }
        let (chunks, _) = v.as_chunks::<6>();
        Ok(PeerAddresses(
            chunks
                .iter()
                .map(|&[a, b, c, d, p1, p2]| {
                    SocketAddrV4::new(Ipv4Addr::new(a, b, c, d), u16::from_be_bytes([p1, p2]))
                })
                .collect(),
        ))
    }
}

impl<'de> Deserialize<'de> for PeerAddresses {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(PeerAddressesVisitor)
    }
}

impl Serialize for PeerAddresses {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut single_slice = Vec::with_capacity(6 * self.0.len());
        for peer in &self.0 {
            single_slice.extend(peer.ip().octets());
            single_slice.extend(peer.port().to_be_bytes());
        }
        serializer.serialize_bytes(&single_slice)
    }
}
