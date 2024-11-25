use serde_bencode::de;
use serde_bytes::ByteBuf;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct File {
    pub path: Vec<String>,
    pub length: i64,
}

#[derive(Debug, Deserialize, Default)]
pub struct Info {
    pub name: String,
    pub pieces: ByteBuf,
    #[serde(rename = "piece length")]
    pub piece_length: i64,
    #[serde(default)]
    pub length: Option<i64>,
    #[serde(default)]
    pub files: Option<Vec<File>>,
    #[serde(default)]
    #[serde(rename = "root hash")]
    pub root_hash: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Torrent {
    pub info: Info,
    #[serde(default)]
    pub announce: Option<String>,
    #[serde(default)]
    #[serde(rename = "announce-list")]
    pub announce_list: Option<Vec<Vec<String>>>,
    #[serde(default)]
    #[serde(rename = "creation date")]
    pub creation_date: Option<i64>,
}

#[derive(thiserror::Error, Debug)]
pub enum TorrentError {
    #[error("Bencode decoding error: {0}")]
    BencodeDecoding(#[from] serde_bencode::Error),
}

pub fn decode(buffer: &[u8]) -> Result<Torrent, TorrentError> {
    Ok(de::from_bytes::<Torrent>(buffer)?)
}
