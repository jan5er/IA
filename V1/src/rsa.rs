use num_bigint::BigUint;
use num_bigint::BigInt;
use crate::lcg;
use crate::prime_tests;
use crate::math;
use crate::bits;

use std::fs::File;
use std::io::ErrorKind::OutOfMemory;
use std::io::Write;
use std::u64;

pub struct RsaKeyValues {
    pub n: BigUint,
    pub e: BigUint,
    pub d: BigUint,
}

pub enum RsaKeyType {
    PublicKey,
    PrivateKey,
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

pub fn save_rsa_key_to_file(rsa_keys: &RsaKeyValues, rsa_key_type: RsaKeyType) {
    let (path, exponent) = match rsa_key_type {
        RsaKeyType::PublicKey => ("pubkey.txt", &rsa_keys.e),
        RsaKeyType::PrivateKey => ("privkey.txt", &rsa_keys.d),
    };

    let mut file = File::create(path).expect("Unable to create file");
    writeln!(file, "{},{}", exponent, rsa_keys.n).expect("Unable to write to file");
}

pub fn read_rsa_key_from_file(path: &str) -> (BigUint, BigUint) {
    let text = std::fs::read_to_string(path).expect("Unable to read file");
    let (exp, n) = text.trim().split_once(',').expect("Invalid key file");
    (exp.parse().expect("Invalid exponent"), n.parse().expect("Invalid modulus"))
}

pub fn encrypt_file(input_path: &str, output_path: &str, rsa_keys_path: &str) {
    let input_data = std::fs::read(input_path).expect("Unable to read input file");
    let (e, n) = read_rsa_key_from_file(rsa_keys_path);

    let read_bits_size = (n.bits() - 1) as usize;
    let write_bits_size = n.bits() as usize;

    let mut bit_reader = crate::bits::BitReader::new(&input_data);
    let mut bit_writer = crate::bits::BitWriter::new();

    while bit_reader.has_more() {
        let m = bit_reader.read_bits(read_bits_size);
        let c = math::modular_exponetiation(&m, &e, &n);
        bit_writer.push_bits(&c, write_bits_size);
    }

    let ext = std::path::Path::new(input_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let ext_bytes = ext.as_bytes();

    let mut out = Vec::new();
    out.push(ext_bytes.len() as u8);                              // 1B za velikost extension tipa
    out.extend_from_slice(ext_bytes);                              // Extension tip
    out.extend_from_slice(&(input_data.len() as u64).to_be_bytes()); // 8B za velikost originala
    out.extend(bit_writer.finish());

    std::fs::write(output_path, out).expect("Unable to write output file");
}

pub fn decrypt_file(input_path: &str, output_path: &str, rsa_keys_path: &str) {
    let input_data = std::fs::read(input_path).expect("Unable to read input file");
    let (d, n) = read_rsa_key_from_file(rsa_keys_path);

    let encrypted_bits_size = n.bits() as usize;   
    let write_size = (n.bits() - 1) as usize;   

    // Header: [1B ext length][ext][8B original size]
    let ext_len = input_data[0] as usize;
    let ext = String::from_utf8_lossy(&input_data[1..1 + ext_len]).to_string();
    let size_start = 1 + ext_len;
    let original_size = u64::from_be_bytes(input_data[size_start..size_start + 8].try_into().unwrap()) as usize;
    let data_start = size_start + 8;

    let total_chunks = (original_size * 8 + write_size - 1) / write_size; 

    let mut bit_reader = crate::bits::BitReader::new(&input_data[data_start..]); // Preskočimo cel header
    let mut bit_writer = crate::bits::BitWriter::new(); 

    for _ in 0..total_chunks {
        let c = bit_reader.read_bits(encrypted_bits_size);
        let m = math::modular_exponetiation(&c, &d, &n);
        bit_writer.push_bits(&m, write_size);
    }

    let output_path = if ext.is_empty() {
        output_path.to_string()
    } else {
        format!("{}.{}", output_path, ext)
    };

    let mut out = bit_writer.finish();
    out.truncate(original_size); // Odstranimo morebitne dodatne bite, ki so nastali zaradi padding-a
    std::fs::write(output_path, out).expect("Unable to write output file");
}

