// N digits -> interval na [X^(N-1), X^N - 1] 
// Super–Duper. Ta je definiran z naslednjimi konstantami: LCG(232, 69069, 1, 0)
// use num_bigint::BigUint;
use num_bigint::BigUint;

#[derive(Clone)]
pub struct Lcg {
    state: u32,     // state je naš trenuten Ri
}

impl Lcg {
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    pub fn next(&mut self) -> u32 {
        let modulo: u64 = 1u64 << 32; // 2^32
        let multiplier: u64 = 69069;

        let next_state = (multiplier * self.state as u64 + 1) % modulo; // LCG(232, 69069, 1, 0) -> Rn = (69069 * Rn-1 + 1) mod 2^32
        self.state = next_state as u32;
        return self.state;
    }

    pub fn generate_candidate(&mut self, bits_size: usize) -> BigUint {
        let mut candidate: BigUint = BigUint::from(0u32);

        let number_of_chunks = (bits_size + 31) / 32; // Minimalno 1 bit mora biti zato + 31 in overflow za zaokroževanje navzgor

        for _ in 0..number_of_chunks {
            candidate <<= 32;
            candidate += BigUint::from(self.next()); 
        }

        // Obdržimo ZGORNJE bite zlepljenega števila. Spodnji biti LCG-ja z modulom 2^32 imajo zelo kratko periodo
        // (bit 0 se samo izmenjuje, bit 1 ima periodo 4, ...), zgornji pa so statistično najboljši.
        /*
        bits_size = 4, number_of_chunks = 1
        candidate:  01011100 01111100 01010000 10011100
        >> (32 - 4):                                0101
        */
        candidate >>= number_of_chunks * 32 - bits_size;

        candidate |= BigUint::from(1u32) << (bits_size - 1); // Nastavimo MSb na 1 da 100% dobimo bits_size-bitno število
        candidate |= BigUint::from(1u32); // Nastavimo še LSb na 1 da dobimo liho število in potem dejansko ne rabimo še enega checka pozneje

        return candidate;
    }

    // Za Miller-Rabina potrebujemo naključno število a, ki pa je lahko tudi sodo zato dejansko samo vzamem prvo polovico zgornje funkcije
    pub fn generate_random_bits(&mut self, bits: usize) -> BigUint {
        let mut number = BigUint::from(0u32);

        let chunks = (bits + 31) / 32;

        for _ in 0..chunks {
            number <<= 32;
            number += BigUint::from(self.next());
        }

        return number >> (chunks * 32 - bits); // zgornji biti, glej generate_candidate
    }

    /// RANDOM(x, y) iz navodil za intervale, ki ustrezajo 32 bitom: x + LCG() mod (y-x+1)
    pub fn random_u32(&mut self, min: u32, max: u32) -> u32 {
        let range = max as u64 - min as u64 + 1;
        return (min as u64 + self.next() as u64 % range) as u32;
    }

    /// RANDOM(x, y) za poljubno velika števila: x + LCG() mod (y-x+1), kjer za velike intervale
    /// zlepimo dovolj 32-bitnih delov (vsaj 64 bitov več kot interval), da je ostanek enakomerno porazdeljen.
    pub fn random(&mut self, min: &BigUint, max: &BigUint) -> BigUint {
        let range = max - min + BigUint::from(1u32);
        let chunks = (range.bits() as usize + 31) / 32 + 2;

        let mut number = BigUint::from(0u32);
        for _ in 0..chunks {
            number <<= 32;
            number += BigUint::from(self.next());
        }

        return min + (number % range);
    }
}
