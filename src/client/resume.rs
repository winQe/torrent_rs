use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_derive::{Deserialize, Serialize};

use crate::message::PieceIndex;

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumeData {
    pub info_hash: [u8; 20],
    pub completed_pieces: Vec<PieceIndex>,
    pub downloaded_bytes: u64,
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
