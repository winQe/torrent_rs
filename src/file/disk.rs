use anyhow::Context;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use super::FileManager;
use crate::message::PieceIndex;

pub struct DiskFileManager {
    files: Vec<std::fs::File>,
    piece_size: u32,
    file_info: Vec<(String, u64)>,
}

impl FileManager for DiskFileManager {
    fn new(base_path: PathBuf, files: Vec<(String, u64)>, piece_size: u32) -> anyhow::Result<Self> {
        let mut file_handles = Vec::with_capacity(files.len());

        fs::create_dir_all(&base_path).context("Failed to create download directory")?;

        for (filename, _) in &files {
            let file_path = base_path.join(filename);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).context("Failed to create parent directory")?;
            }

            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&file_path)
                .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
            file_handles.push(file);
        }

        Ok(Self {
            files: file_handles,
            file_info: files,
            piece_size,
        })
    }

    // Writes the downloaded piece to disk directly, also handles the case where one piece might be
    // split into multiple files
    fn write_piece(&mut self, piece_index: PieceIndex, data: &[u8]) -> anyhow::Result<()> {
        let piece_offset = piece_index as u64 * self.piece_size as u64;
        let mut current_offset = piece_offset;
        // These data could be split into multiple files, need to keep track which one we have
        // written
        let mut remaining_data = data;

        // Find which file(s) this piece spans
        let mut file_offset = 0u64;

        for (file_idx, (_, file_size)) in self.file_info.iter().enumerate() {
            if current_offset < file_offset + file_size {
                // This file contains part of our piece
                let file_start = if current_offset > file_offset {
                    current_offset - file_offset
                } else {
                    // Just at the start of the file
                    0
                };

                // How many bytes of data we should write to this particular file
                let bytes_in_this_file =
                    std::cmp::min(remaining_data.len() as u64, file_size - file_start) as usize;

                if bytes_in_this_file > 0 {
                    self.files[file_idx]
                        .seek(SeekFrom::Start(file_start))
                        .context("Error seeking file")?;
                    self.files[file_idx]
                        .write_all(&remaining_data[..bytes_in_this_file])
                        .context("Failed to write buffer")?;

                    remaining_data = &remaining_data[bytes_in_this_file..];
                    current_offset += bytes_in_this_file as u64;

                    // All data written already
                    if remaining_data.is_empty() {
                        break;
                    }
                }
            }

            file_offset += file_size;
        }

        Ok(())
    }

    fn read_piece(&mut self, piece_index: PieceIndex, length: u32) -> anyhow::Result<Vec<u8>> {
        let piece_offset = piece_index as u64 * self.piece_size as u64;
        let mut current_offset = piece_offset;
        let mut buf = vec![0u8; length as usize];
        let mut written = 0usize;

        let mut file_offset = 0u64;
        for (file_idx, (_, file_size)) in self.file_info.iter().enumerate() {
            if current_offset < file_offset + file_size {
                let file_start = current_offset.saturating_sub(file_offset);
                let bytes_in_this_file = std::cmp::min(
                    (length as usize) - written,
                    (file_size - file_start) as usize,
                );

                if bytes_in_this_file > 0 {
                    self.files[file_idx]
                        .seek(SeekFrom::Start(file_start))
                        .context("Error seeking file")?;
                    self.files[file_idx]
                        .read_exact(&mut buf[written..written + bytes_in_this_file])
                        .context("Failed to read piece bytes")?;

                    written += bytes_in_this_file;
                    current_offset += bytes_in_this_file as u64;

                    if written == length as usize {
                        break;
                    }
                }
            }
            file_offset += file_size;
        }

        Ok(buf)
    }
}
