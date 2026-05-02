use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_derive::{Deserialize, Serialize};

use crate::message::PieceIndex;

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumeData {
    pub info_hash: [u8; 20],
    pub total_pieces: u32,
    pub piece_bitfield: Vec<u8>,
    pub downloaded_bytes: u64,
}

impl ResumeData {
    pub fn from_completed(
        info_hash: [u8; 20],
        completed: &HashSet<PieceIndex>,
        total_pieces: u32,
        downloaded_bytes: u64,
    ) -> Self {
        let bytes_needed = total_pieces.div_ceil(8) as usize;
        let mut bitfield = vec![0u8; bytes_needed];
        for &idx in completed {
            if idx < total_pieces {
                bitfield[(idx / 8) as usize] |= 0x80 >> (idx % 8);
            }
        }
        Self {
            info_hash,
            total_pieces,
            piece_bitfield: bitfield,
            downloaded_bytes,
        }
    }

    pub fn completed_pieces(&self) -> Vec<PieceIndex> {
        let mut result = Vec::new();
        for i in 0..self.total_pieces {
            let byte_idx = (i / 8) as usize;
            let bit_mask = 0x80u8 >> (i % 8);
            if byte_idx < self.piece_bitfield.len() && self.piece_bitfield[byte_idx] & bit_mask != 0
            {
                result.push(i);
            }
        }
        result
    }
}

pub fn resume_path(download_dir: &Path, info_hash: &[u8; 20]) -> PathBuf {
    download_dir.join(format!(".{}.resume", hex::encode(info_hash)))
}

pub fn load(path: &Path) -> Result<Option<ResumeData>> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let data: ResumeData = serde_json::from_slice(&bytes)
                .with_context(|| format!("Failed to parse resume file: {}", path.display()))?;
            Ok(Some(data))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("Failed to read resume file: {}", path.display())),
    }
}

pub fn save(path: &Path, data: &ResumeData) -> Result<()> {
    let tmp = path.with_extension("resume.tmp");
    let bytes = serde_json::to_vec(data).context("Failed to serialize resume data")?;
    std::fs::write(&tmp, bytes)
        .with_context(|| format!("Failed to write resume tmp file: {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("Failed to rename resume file to: {}", path.display()))?;
    Ok(())
}
