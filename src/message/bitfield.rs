use super::PieceIndex;

#[derive(Debug)]
pub struct Bitfield {
    pub data: Vec<u8>,
}

impl Bitfield {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { data: bytes }
    }

    pub fn empty(total_pieces: usize) -> Self {
        Self {
            data: vec![0u8; total_pieces.div_ceil(8)],
        }
    }

    pub fn has_piece(&self, index: usize) -> bool {
        let byte_index = index / 8;
        let bit_index = index % 8;

        // Out of bounds check, usize can't be negative
        if byte_index >= self.data.len() {
            return false;
        }

        // Big endian bit ordering
        self.data[byte_index] & (1 << (7 - bit_index)) != 0
    }

    pub fn set_piece(&mut self, index: usize) {
        let byte_index = index / 8;
        let bit_index = index % 8;
        if byte_index < self.data.len() {
            self.data[byte_index] |= 1 << (7 - bit_index);
        }
    }

    pub fn len(&self) -> usize {
        self.data.len() * 8
    }

    pub fn iter(&self) -> BitfieldIterator {
        BitfieldIterator {
            bitfield: self,
            index: 0,
        }
    }
}

pub struct BitfieldIterator<'a> {
    bitfield: &'a Bitfield,
    index: PieceIndex,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bitfield_has_no_pieces() {
        let bf = Bitfield::empty(13);
        assert_eq!(bf.data.len(), 2); // ceil(13/8)
        for i in 0..13 {
            assert!(!bf.has_piece(i));
        }
    }

    #[test]
    fn set_piece_marks_only_requested_index() {
        let mut bf = Bitfield::empty(16);
        bf.set_piece(5);
        for i in 0..16 {
            assert_eq!(bf.has_piece(i), i == 5);
        }
    }

    #[test]
    fn set_piece_out_of_range_is_noop() {
        let mut bf = Bitfield::empty(8);
        bf.set_piece(100);
        for i in 0..8 {
            assert!(!bf.has_piece(i));
        }
    }
}

impl<'a> Iterator for BitfieldIterator<'a> {
    type Item = PieceIndex;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.bitfield.len() as u32 {
            let current = self.index;
            self.index += 1;

            if self.bitfield.has_piece(current as usize) {
                return Some(current as PieceIndex);
            }
        }
        None
    }
}
