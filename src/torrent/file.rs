use anyhow::Result;
use std::fs;
use std::path::Path;

use super::bencode::{decode, Torrent};

pub fn open<P: AsRef<Path>>(path: P) -> Result<Torrent> {
    let buf = fs::read(path)?;
    Ok(decode(&buf)?)
}
