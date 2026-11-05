mod bench;
mod bits;
mod cli;
mod gui;
mod lcg;
mod math;
mod prime_tests;
mod rsa;

fn main() -> eframe::Result {
    // `cargo run -- --cli` zažene prvotni konzolni meni, sicer se odpre grafični vmesnik
    if std::env::args().any(|arg| arg == "--cli") {
        cli::run(0);
        return Ok(());
    }

    gui::run()
}

#[cfg(test)]
mod tests {
    use crate::lcg::Lcg;
    use crate::prime_tests::{miller_rabin_test, naive_is_prime, MillerRabinResult};
    use crate::rsa::{self, PrimeMethod};
    use num_bigint::BigUint;

    #[test]
    fn naive_and_miller_rabin_agree_on_small_numbers() {
        let mut lcg = Lcg::new(1);
        for n in 0u32..5000 {
            let big = BigUint::from(n);
            let naive = naive_is_prime(&big);
            let mr = !matches!(miller_rabin_test(&mut lcg, big, 10), MillerRabinResult::CompositeNumber);
            assert_eq!(naive, mr, "disagreement for {}", n);
        }
    }

    #[test]
    fn generators_return_primes_with_exact_bit_length() {
        let mut lcg = Lcg::new(7);
        for bits in 2..=32 {
            for method in [PrimeMethod::Naive, PrimeMethod::MillerRabin] {
                let p = rsa::generate_prime(&mut lcg, bits, method, 10);
                assert_eq!(p.bits() as usize, bits, "{:?} {} bits gave {}", method, bits, p);
                assert!(naive_is_prime(&p), "{:?} {} bits gave composite {}", method, bits, p);
            }
        }
    }

    #[test]
    fn rsa_round_trip_for_all_key_sizes() {
        let mut lcg = Lcg::new(3);
        let data: Vec<u8> = (0..2000u32).map(|i| (i * 31 % 251) as u8).collect();
        for bits in [3usize, 4, 5, 8, 12, 15, 32, 64, 256] {
            for method in [PrimeMethod::Naive, PrimeMethod::MillerRabin] {
                if method == PrimeMethod::Naive && bits > 32 { continue; }
                let keys = rsa::key_generator(&mut lcg, bits, method, 10).unwrap_or_else(|e| panic!("{:?} {} bits: {}", method, bits, e));
                let enc = rsa::encrypt_bytes(&data, "bin", &keys.e, &keys.n);
                let (dec, ext) = rsa::decrypt_bytes(&enc, &keys.d, &keys.n).unwrap();
                assert_eq!(dec, data, "round trip failed for {:?} {} bits", method, bits);
                assert_eq!(ext, "bin");
            }
        }
    }

    #[test]
    fn decrypt_rejects_garbage() {
        let n = BigUint::from(35u32);
        assert!(rsa::decrypt_bytes(&[], &n, &n).is_err());
        assert!(rsa::decrypt_bytes(&[200, 1, 2], &n, &n).is_err());
        assert!(rsa::decrypt_bytes(&[0, 0, 0, 0, 0, 0, 0, 255, 255, 1], &n, &n).is_err());
    }

    #[test]
    fn random_stays_inside_interval() {
        let mut lcg = Lcg::new(0);
        for _ in 0..10_000 {
            let v = lcg.random(&BigUint::from(10u32), &BigUint::from(100u32));
            assert!(v >= BigUint::from(10u32) && v <= BigUint::from(100u32));
            let w = lcg.random_u32(5, 7);
            assert!((5..=7).contains(&w));
        }
    }
}
