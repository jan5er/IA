use num_bigint::BigUint;
use num_bigint::BigInt;
use crate::lcg;
use crate::prime_tests;
use crate::math;

pub struct RsaKeyValues {
    pub n: BigUint,
    pub e: BigUint,
    pub d: BigUint,
}


pub fn key_generator(lcg: &mut lcg::Lcg) -> RsaKeyValues {
    let one = BigUint::from(1u32);
    let two = BigUint::from(2u32);
    
    let (prime1, prime2) = loop {
        let prime1 = prime_tests::miller_rabin_prime_generator(lcg, 10, 10);
        let prime2 = prime_tests::miller_rabin_prime_generator(lcg, 10, 10);

        if prime1 != prime2 { break (prime1, prime2); }
    };

    let n: BigUint = &prime1 * &prime2;
    let phi_n: BigUint = (&prime1 - &one) * (&prime2 - &one);

    let (d, e) = loop {
        let mut e_candidate = lcg.random(&two, &(&phi_n - &one)); // (1, phi_n) -> [2, phi_n - 1]
        e_candidate |= &one; // LSb na 1 za liho število 
        if math::gcd(&e_candidate, &phi_n) == one && e_candidate < phi_n { // ker nastavimo LSb na 1 mogoče lahko pride do e_candidate == phi_n zato še en check
            let d = math::modular_linear_equation_solver(&e_candidate, &one, &phi_n).expect("No modular inverse found for e and phi_n");
            break (d, e_candidate);
        }
    };

    return RsaKeyValues { n, e, d };
}

