#[derive(Debug)]
pub struct Bitfield {
    data: Vec<u8>,
}

impl Bitfield {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { data: bytes }
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
}
