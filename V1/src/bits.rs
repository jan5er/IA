use num_bigint::BigUint;

pub struct BitWriter {
    bytes: Vec<u8>,
    current_byte: u8,
    n_bits: u8,
}

pub struct BitReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl BitWriter {
    pub fn new() -> Self {
        BitWriter {
            bytes: Vec::new(),
            current_byte: 0,
            n_bits: 0,
        }
    }

    pub fn push_bits(&mut self, value: &BigUint, bits_size: usize) {
        for i in (0..bits_size).rev() {
            // Začnemo z MSb in gremo proti LSb - torej prva iteracija bo i = bits_size - 1
            /*
            let bit = (value >> i) & BigUint::from(1u32); // 1110101 >> size = 0000001 & 1 = 1
             */
            let bit = value.bit(i as u64) as u8;
            self.current_byte = (self.current_byte << 1) | bit; // Current_byte shiftamo levo in dodamo bit na current LSb (desni strani)
            self.n_bits += 1;

            if self.n_bits == 8 {
                // Ko dosežemo byte flushamo
                self.bytes.push(self.current_byte);
                self.current_byte = 0;
                self.n_bits = 0;
            }
        }
    }

    pub fn finish(mut self) -> Vec<u8> {
        if self.n_bits > 0 {
            self.current_byte <<= 8 - self.n_bits; // Shiftamo preostale bite na levo
            self.bytes.push(self.current_byte);
        }
        self.bytes
    }
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        BitReader {
            data,
            position: 0,
        }
    }

    pub fn read_bits(&mut self, bits_size: usize) -> BigUint {
        let mut value = BigUint::from(0u32);

        for _ in 0..bits_size {
            let byte_index = self.position / 8; // Določimo kateri byte beremo
            let bit = if byte_index < self.data.len() { // Če še nismo presegli dolžine podatkov preberemo bit
                (self.data[byte_index] >> (7 - self.position % 8)) & 1 // 
            } else {
                0
            };

            value = (value << 1) | BigUint::from(bit);
            self.position += 1;
        }

        value
    }

    pub fn has_more(&self) -> bool {
        self.position < self.data.len() * 8
    }

}

pub fn test_bit_writer() {
    let mut w = BitWriter::new();
    w.push_bits(&BigUint::from(5u32), 3);
    w.push_bits(&BigUint::from(51u32), 6);
    assert_eq!(w.finish(), vec![0xB9, 0x80]);
}