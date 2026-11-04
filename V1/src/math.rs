use std::arch::x86_64;

use num_bigint::{BigInt, BigUint};

pub fn modular_exponetiation(base: &BigUint, exponent: &BigUint, modulus: &BigUint) -> BigUint {
    let mut d = BigUint::from(1u32);
    let binary_exponent = exponent.to_str_radix(2); 

    for bit in binary_exponent.chars() {
        d = (&d * &d) % modulus;

        if bit == '1' {
            d = (&d * base) % modulus;
        }
    }

    return d;
}

pub fn gcd(a: &BigUint, b: &BigUint) -> BigUint {
    // Gre za deljenje a/b
    let (mut a, mut b) = (a.clone(), b.clone());
    let zero = BigUint::from(0u32);

    while b != zero {
        let remainder = &a % &b;
        a = b;
        b = remainder;
    }

    return a;
}

pub fn extended_euclid(a: &BigInt, b: &BigInt) -> (BigInt, BigInt, BigInt) {
                                                //   d,      x,      y
    if *b == BigInt::from(0) {
        let d = a.clone(); 
        let x = BigInt::from(1);
        let y = BigInt::from(0);
        return (d, x, y);
    } else {
        let (d, n_x, n_y) = extended_euclid(b, &(a % b));
        return (d, n_y.clone(), n_x - (a / b) * n_y);
    }
}

pub fn modular_linear_equation_solver(a: &BigUint, b: &BigUint, n: &BigUint) -> Option<BigUint> {
    let (d, x, y) = extended_euclid(&BigInt::from(a.clone()), &BigInt::from(n.clone()));
    let zero = BigInt::from(0);
    let b = BigInt::from(b.clone());
    let n = BigInt::from(n.clone());

    if &d % &b == zero {
        let x0 = (&x * (&b / &d)) % &n; // x0 je lahko negativno

        /* Dejansko ne loopa do d ker nas zanima samo prvi rezultat
        for i in 0..(d - 1) {
            println!("x{} = {}", i, (&x0 + i * (n / &d)) % n);
        }
        */

        // Če je x0 negativno potem ga moramo še pretvoriti v pozitivno število
        // x0: ( x0=-17 + 40 ) % 40 = 23
        return ((&x0 + &n) % &n).to_biguint(); 
    } else {
        return None;
    }
}
