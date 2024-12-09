use anyhow::Context;
use core::fmt;
use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::path::Path;

mod hashes;

pub use hashes::Hashes;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Torrent {
    /// The URL of the tracker.
    pub announce: String,
    pub info: Info,
    pub info_hash: Option<[u8; 20]>,
}

impl Torrent {
    pub fn get_info_hash(&mut self) -> anyhow::Result<()> {
        if self.info_hash.is_some() {
            return Ok(());
        }

        let info_encoded =
            serde_bencode::to_bytes(&self.info).context("Failed to re-encode info torrent")?;

        let mut hasher = Sha1::new();
        hasher.update(&info_encoded);

        let hash: [u8; 20] = hasher.finalize().into();

        self.info_hash = Some(hash);

        Ok(())
    }

    pub fn urlencode_infohash(&self) -> Option<String> {
        self.info_hash.map(|info_hash| {
            let mut encoded = String::with_capacity(info_hash.len() * 3);
            info_hash.into_iter().for_each(|byte| {
                encoded.push('%');
                encoded.push_str(&format!("{:02X}", byte));
            });
            encoded
        })
    }
    #[tracing::instrument]
    pub async fn open(file: impl AsRef<Path> + fmt::Debug) -> anyhow::Result<Self> {
        let file = tokio::fs::read(file)
            .await
            .context("Failed opening torrent file")?;
        let mut t: Torrent =
            serde_bencode::from_bytes(&file).context("Failed parsing torrent file")?;
        t.get_info_hash().context("Failed to get info hash")?;

        tracing::info!("Succesfully opened {}", t.info.name);
        Ok(t)
    }

    pub fn print_tree(&self) {
        match &self.info.keys {
            Keys::SingleFile { .. } => {
                eprintln!("{}", self.info.name);
            }
            Keys::MultiFile { files } => {
                for file in files {
                    eprintln!("{}", file.path.join(std::path::MAIN_SEPARATOR_STR));
                }
            }
        }
    }

    pub fn length(&self) -> usize {
        match &self.info.keys {
            Keys::SingleFile { length } => *length,
            Keys::MultiFile { files } => files.iter().map(|file| file.length).sum(),
        }
    }
}

// Structure mainly from https://github.com/jonhoo/codecrafters-bittorrent-rust/blob/master/src/torrent.rs
// to ensure info hash is correct

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Info {
    /// The suggested name to save the file (or directory) as. It is purely advisory.
    ///
    /// In the single file case, the name key is the name of a file, in the muliple file case, it's
    /// the name of a directory.
    pub name: String,

    /// The number of bytes in each piece the file is split into.
    ///
    /// For the purposes of transfer, files are split into fixed-size pieces which are all the same
    /// length except for possibly the last one which may be truncated. piece length is almost
    /// always a power of two, most commonly 2^18 = 256K (BitTorrent prior to version 3.2 uses 2
    /// 20 = 1 M as default).
    #[serde(rename = "piece length")]
    pub piece_length: usize,

    /// Each entry of `pieces` is the SHA1 hash of the piece at the corresponding index.
    pub pieces: Hashes,

    #[serde(flatten)]
    pub keys: Keys,
}

/// There is a key `length` or a key `files`, but not both or neither.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Keys {
    /// If `length` is present then the download represents a single file.
    SingleFile {
        /// The length of the file in bytes.
        length: usize,
    },
    /// Otherwise it represents a set of files which go in a directory structure.
    ///
    /// For the purposes of the other keys in `Info`, the multi-file case is treated as only having
    /// a single file by concatenating the files in the order they appear in the files list.
    MultiFile { files: Vec<File> },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct File {
    /// The length of the file, in bytes.
    pub length: usize,

    /// Subdirectory names for this file, the last of which is the actual file name
    /// (a zero length list is an error case).
    pub path: Vec<String>,
}
