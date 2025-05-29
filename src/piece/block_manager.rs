#![allow(dead_code)]
use super::{Block, BlockInfo, PieceIndex, BLOCK_SIZE};
use std::collections::HashMap;

pub struct BlockManager {
    piece_blocks: HashMap<PieceIndex, Vec<Option<Block>>>,
    pending_blocks: HashMap<BlockInfo, std::time::Instant>,
}

impl BlockManager {
    pub fn new() -> Self {
        Self {
            piece_blocks: HashMap::new(),
            pending_blocks: HashMap::new(),
        }
    }

    pub fn init_piece(&mut self, piece_index: PieceIndex, piece_size: u32) {
        let num_blocks = piece_size.div_ceil(BLOCK_SIZE);
        self.piece_blocks
            .insert(piece_index, vec![None; num_blocks as usize]);
    }

    pub fn next_block(&mut self, piece_index: PieceIndex, piece_size: u32) -> Option<BlockInfo> {
        let blocks = self.piece_blocks.get(&piece_index)?;

        for (i, block) in blocks.iter().enumerate() {
            if block.is_none() {
                let offset = i as u32 * BLOCK_SIZE;
                let length = std::cmp::min(BLOCK_SIZE, piece_size - offset);
                let block_info = BlockInfo {
                    piece_index,
                    offset,
                    length,
                };

                if !self.pending_blocks.contains_key(&block_info) {
                    self.pending_blocks
                        .insert(block_info, std::time::Instant::now());
                    return Some(block_info);
                }
            }
        }
        None
    }

    pub fn store_block(&mut self, block_info: BlockInfo, data: Block) {
        self.pending_blocks.remove(&block_info);

        if let Some(block) = self.piece_blocks.get_mut(&block_info.piece_index) {
            let block_index = (block_info.offset / BLOCK_SIZE) as usize;
            if block_index < block.len() {
                block[block_index] = Some(data);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create test data
    fn create_test_block(size: usize) -> Block {
        vec![0u8; size]
    }

    #[test]
    fn test_new() {
        let manager = BlockManager::new();
        assert!(manager.piece_blocks.is_empty());
        assert!(manager.pending_blocks.is_empty());
    }

    #[test]
    fn test_init_piece_single_block() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = 8192; // Half of BLOCK_SIZE

        manager.init_piece(piece_index, piece_size);

        assert!(manager.piece_blocks.contains_key(&piece_index));
        let blocks = manager.piece_blocks.get(&piece_index).unwrap();
        assert_eq!(blocks.len(), 1); // Should have 1 block
        assert!(blocks[0].is_none());
    }

    #[test]
    fn test_init_piece_multiple_blocks() {
        let mut manager = BlockManager::new();
        let piece_index = 1;
        let piece_size = BLOCK_SIZE * 2 + 1000; // 2.x blocks

        manager.init_piece(piece_index, piece_size);

        assert!(manager.piece_blocks.contains_key(&piece_index));
        let blocks = manager.piece_blocks.get(&piece_index).unwrap();
        assert_eq!(blocks.len(), 3); // Should have 3 blocks (div_ceil)
        for block in blocks {
            assert!(block.is_none());
        }
    }

    #[test]
    fn test_init_piece_exact_block_size() {
        let mut manager = BlockManager::new();
        let piece_index = 2;
        let piece_size = BLOCK_SIZE;

        manager.init_piece(piece_index, piece_size);

        let blocks = manager.piece_blocks.get(&piece_index).unwrap();
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn test_init_piece_overwrites_existing() {
        let mut manager = BlockManager::new();
        let piece_index = 0;

        // Initialize with one size
        manager.init_piece(piece_index, BLOCK_SIZE);
        assert_eq!(manager.piece_blocks.get(&piece_index).unwrap().len(), 1);

        // Initialize with different size - should overwrite
        manager.init_piece(piece_index, BLOCK_SIZE * 3);
        assert_eq!(manager.piece_blocks.get(&piece_index).unwrap().len(), 3);
    }

    #[test]
    fn test_next_block_piece_not_initialized() {
        let mut manager = BlockManager::new();
        let piece_index = 99;
        let piece_size = BLOCK_SIZE;

        let result = manager.next_block(piece_index, piece_size);
        assert!(result.is_none());
    }

    #[test]
    fn test_next_block_first_block() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE;

        manager.init_piece(piece_index, piece_size);

        let block_info = manager.next_block(piece_index, piece_size).unwrap();
        assert_eq!(block_info.piece_index, piece_index);
        assert_eq!(block_info.offset, 0);
        assert_eq!(block_info.length, BLOCK_SIZE);

        // Should be marked as pending
        assert!(manager.pending_blocks.contains_key(&block_info));
    }

    #[test]
    fn test_next_block_multiple_blocks() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE * 2 + 1000;

        manager.init_piece(piece_index, piece_size);

        // Get first block
        let block1 = manager.next_block(piece_index, piece_size).unwrap();
        assert_eq!(block1.offset, 0);
        assert_eq!(block1.length, BLOCK_SIZE);

        // Get second block
        let block2 = manager.next_block(piece_index, piece_size).unwrap();
        assert_eq!(block2.offset, BLOCK_SIZE);
        assert_eq!(block2.length, BLOCK_SIZE);

        // Get third block (partial)
        let block3 = manager.next_block(piece_index, piece_size).unwrap();
        assert_eq!(block3.offset, BLOCK_SIZE * 2);
        assert_eq!(block3.length, 1000); // Remaining bytes
    }

    #[test]
    fn test_next_block_no_more_blocks() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE;

        manager.init_piece(piece_index, piece_size);

        // Get the only block
        let _block = manager.next_block(piece_index, piece_size).unwrap();

        // Try to get another block - should return None
        let result = manager.next_block(piece_index, piece_size);
        assert!(result.is_none());
    }

    #[test]
    fn test_next_block_skips_pending() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE * 2;

        manager.init_piece(piece_index, piece_size);

        // Get first block (becomes pending)
        let block1 = manager.next_block(piece_index, piece_size).unwrap();

        // Get next block - should be the second block since first is pending
        let block2 = manager.next_block(piece_index, piece_size).unwrap();
        assert_eq!(block2.offset, BLOCK_SIZE);

        // First block should still be pending
        assert!(manager.pending_blocks.contains_key(&block1));
    }

    #[test]
    fn test_store_block_valid() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE;

        manager.init_piece(piece_index, piece_size);
        let block_info = manager.next_block(piece_index, piece_size).unwrap();

        let test_data = create_test_block(BLOCK_SIZE as usize);
        manager.store_block(block_info.clone(), test_data.clone());

        // Block should no longer be pending
        assert!(!manager.pending_blocks.contains_key(&block_info));

        // Block should be stored
        let blocks = manager.piece_blocks.get(&piece_index).unwrap();
        assert!(blocks[0].is_some());
        assert_eq!(blocks[0].as_ref().unwrap(), &test_data);
    }

    #[test]
    fn test_store_block_multiple_blocks() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE * 3;

        manager.init_piece(piece_index, piece_size);

        // Get and store first block
        let block1 = manager.next_block(piece_index, piece_size).unwrap();
        assert_eq!(block1.offset, 0);
        let data1 = create_test_block(BLOCK_SIZE as usize);
        manager.store_block(block1, data1.clone());

        // Get and store second block (skip storing to test gaps)
        let block2 = manager.next_block(piece_index, piece_size).unwrap();
        assert_eq!(block2.offset, BLOCK_SIZE);

        // Get and store third block
        let block3 = manager.next_block(piece_index, piece_size).unwrap();
        assert_eq!(block3.offset, BLOCK_SIZE * 2);
        let data3 = create_test_block(BLOCK_SIZE as usize);
        manager.store_block(block3, data3.clone());

        let blocks = manager.piece_blocks.get(&piece_index).unwrap();
        assert!(blocks[0].is_some());
        assert!(blocks[1].is_none()); // Second block retrieved but not stored
        assert!(blocks[2].is_some());
        assert_eq!(blocks[0].as_ref().unwrap(), &data1);
        assert_eq!(blocks[2].as_ref().unwrap(), &data3);

        // block2 should still be pending since we didn't store it
        assert!(manager.pending_blocks.contains_key(&block2));
    }

    #[test]
    fn test_store_block_invalid_piece() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let invalid_piece_index = 99;

        manager.init_piece(piece_index, BLOCK_SIZE);

        let block_info = BlockInfo {
            piece_index: invalid_piece_index,
            offset: 0,
            length: BLOCK_SIZE,
        };

        let test_data = create_test_block(BLOCK_SIZE as usize);
        manager.store_block(block_info, test_data);

        // Should not crash, and piece 0 should remain unchanged
        let blocks = manager.piece_blocks.get(&piece_index).unwrap();
        assert!(blocks[0].is_none());
    }

    #[test]
    fn test_store_block_invalid_offset() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE;

        manager.init_piece(piece_index, piece_size);

        let block_info = BlockInfo {
            piece_index,
            offset: BLOCK_SIZE * 10, // Way beyond piece size
            length: BLOCK_SIZE,
        };

        let test_data = create_test_block(BLOCK_SIZE as usize);
        manager.store_block(block_info, test_data);

        // Should not crash, original block should remain None
        let blocks = manager.piece_blocks.get(&piece_index).unwrap();
        assert!(blocks[0].is_none());
    }

    #[test]
    fn test_store_block_removes_from_pending() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE;

        manager.init_piece(piece_index, piece_size);
        let block_info = manager.next_block(piece_index, piece_size).unwrap();

        // Verify it's pending
        assert!(manager.pending_blocks.contains_key(&block_info));

        let test_data = create_test_block(BLOCK_SIZE as usize);
        manager.store_block(block_info.clone(), test_data);

        // Should be removed from pending
        assert!(!manager.pending_blocks.contains_key(&block_info));
    }

    #[test]
    fn test_pending_blocks_timing() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE;

        manager.init_piece(piece_index, piece_size);

        let before = std::time::Instant::now();
        let block_info = manager.next_block(piece_index, piece_size).unwrap();
        let after = std::time::Instant::now();

        let pending_time = manager.pending_blocks.get(&block_info).unwrap();
        assert!(*pending_time >= before && *pending_time <= after);
    }

    #[test]
    fn test_workflow_complete_piece() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE * 2 + 1000;

        // Initialize piece
        manager.init_piece(piece_index, piece_size);

        let mut block_infos = Vec::new();
        let mut test_data = Vec::new();

        // Get all blocks
        while let Some(block_info) = manager.next_block(piece_index, piece_size) {
            let data = create_test_block(block_info.length as usize);
            test_data.push(data.clone());
            block_infos.push(block_info.clone());
            manager.store_block(block_info, data);
        }

        // Should have 3 blocks
        assert_eq!(block_infos.len(), 3);
        assert_eq!(test_data.len(), 3);

        // Verify all blocks are stored
        let blocks = manager.piece_blocks.get(&piece_index).unwrap();
        for (i, block) in blocks.iter().enumerate() {
            assert!(block.is_some());
            assert_eq!(block.as_ref().unwrap(), &test_data[i]);
        }

        // No pending blocks should remain
        assert!(manager.pending_blocks.is_empty());
    }

    #[test]
    fn test_piece_size_edge_cases() {
        let mut manager = BlockManager::new();

        // Test with piece_size = 0 (edge case)
        manager.init_piece(0, 0);
        let blocks = manager.piece_blocks.get(&0).unwrap();
        assert_eq!(blocks.len(), 0);

        // Test with piece_size = 1
        manager.init_piece(1, 1);
        let blocks = manager.piece_blocks.get(&1).unwrap();
        assert_eq!(blocks.len(), 1);

        let block_info = manager.next_block(1, 1).unwrap();
        assert_eq!(block_info.length, 1);
    }

    #[test]
    fn test_bug_in_next_block_logic() {
        // This test demonstrates the bug in the original code
        // The condition `if !block.is_none()` should be `if block.is_none()`
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE;

        manager.init_piece(piece_index, piece_size);

        let result = manager.next_block(piece_index, piece_size);

        assert!(
            result.is_some(),
            "next_block should return the first empty block"
        );
    }

    #[test]
    fn test_next_block_after_partial_completion() {
        let mut manager = BlockManager::new();
        let piece_index = 0;
        let piece_size = BLOCK_SIZE * 3;

        manager.init_piece(piece_index, piece_size);

        // Get and store first block
        let block1 = manager.next_block(piece_index, piece_size).unwrap();
        let data1 = create_test_block(BLOCK_SIZE as usize);
        manager.store_block(block1, data1);

        // Next block should be the second one (offset = BLOCK_SIZE)
        let block2 = manager.next_block(piece_index, piece_size).unwrap();
        assert_eq!(block2.offset, BLOCK_SIZE);

        // Store second block
        let data2 = create_test_block(BLOCK_SIZE as usize);
        manager.store_block(block2, data2);

        // Next block should be the third one
        let block3 = manager.next_block(piece_index, piece_size).unwrap();
        assert_eq!(block3.offset, BLOCK_SIZE * 2);

        // After storing all blocks, next_block should return None
        let data3 = create_test_block(BLOCK_SIZE as usize);
        manager.store_block(block3, data3);

        let no_more_blocks = manager.next_block(piece_index, piece_size);
        assert!(no_more_blocks.is_none());
    }
}
