use anyhow::Context;
use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
};

use super::FileManager;
use crate::message::PieceIndex;

static BASE_PATH: &str = "/home/avt/Downloads/";

pub struct DiskFileManager {
    files: Vec<File>,
    piece_size: u32,
    file_info: Vec<(String, u64)>,
}

impl FileManager for DiskFileManager {
    fn new(files: Vec<(String, u64)>, piece_size: u32) -> anyhow::Result<Self> {
        let mut file_handles = Vec::with_capacity(files.len());

        for (filename, _) in &files {
            let file =
                File::create(BASE_PATH.to_owned() + filename).context("Failed to create file")?;
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
}
