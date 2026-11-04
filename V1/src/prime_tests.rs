use num_bigint::BigUint;
use crate::lcg;
use crate::math;

pub enum MillerRabinResult {
    Prime,
    ProbablyPrime,
    CompositeNumber,
}

pub fn naive_prime_generator(lcg: &mut lcg::Lcg, bits_size: usize) -> BigUint {
    let zero: BigUint = BigUint::from(0u32);
    // let one = BigUint::from(1u32);
    let two: BigUint = BigUint::from(2u32);
    let three: BigUint = BigUint::from(3u32);

    // Generiraj naključno bits_size-bitno število prime_candidate
    let mut prime_candidate = lcg.generate_candidate(bits_size);

    /* Ne rabimo več ker prime_candidate vedno generiramo kot liho število
    if &prime_candidate % &two == zero {
        prime_candidate += &one;
    }
    */

    loop {
        let mut potential_divisor = three.clone();

        while (&potential_divisor * &potential_divisor) <= prime_candidate {  // sqrt(prime_candidate) = potential_divisor / ^2 -> prime_candidate = potential_divisor^2
            if &prime_candidate % &potential_divisor == zero {
                break;
            }

            potential_divisor += &two;
        }

        if (&potential_divisor * &potential_divisor) > prime_candidate {
            return prime_candidate;
        }

        prime_candidate += &two;
    }
}

pub fn miller_rabin_test(lcg: &mut lcg::Lcg, prime_candidate: BigUint, s: usize) -> MillerRabinResult {
    let zero: BigUint = BigUint::from(0u32);
    let one = BigUint::from(1u32);
    let two: BigUint = BigUint::from(2u32);
    let three: BigUint = BigUint::from(3u32);

    if prime_candidate <= three { return MillerRabinResult::Prime; }
    if &prime_candidate % &two == zero { return MillerRabinResult::CompositeNumber; } 

    // d2^k= p−1
    let mut d: BigUint = &prime_candidate - &one;
    let mut k: usize = 0;
    while &d % &two == zero {
        d = &d / &two;
        k += 1;
    }

    for _ in 1..=s {
        let max = &prime_candidate - &two;
        let a = lcg.random(&two, &max); // Naključno število a v intervalu [2, prime_candidate - 2]
        let mut x: BigUint = math::modular_exponetiation(&a, &d, &prime_candidate); // x = a^d mod prime_candidate

        if x == one { continue; }

        // Če obstaja i, da je a^(d*2^i) kongruenca -1 (mod p) potem je p verjetno praštevilo

        for _ in 0..(k - 1) {
            if x == (&prime_candidate - &one) { break; }
            x = math::modular_exponetiation(&x, &two, &prime_candidate); // x = x^2 mod p
        }

        if x != (&prime_candidate - &one) {
            return MillerRabinResult::CompositeNumber;
        }
    }
    return MillerRabinResult::ProbablyPrime;
}

pub fn miller_rabin_prime_generator(lcg: &mut lcg::Lcg, bits_size: usize, s: usize) -> BigUint {
    let two = BigUint::from(2u32);
    let mut prime_candidate = lcg.generate_candidate(bits_size);

    loop {
        let result = miller_rabin_test(lcg, prime_candidate.clone(), s);

        match result {
            MillerRabinResult::Prime => return prime_candidate,
            MillerRabinResult::ProbablyPrime => return prime_candidate,
            MillerRabinResult::CompositeNumber => prime_candidate += &two,
        }
    }
}