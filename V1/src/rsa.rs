use num_bigint::BigUint;
use crate::bits::{BitReader, BitWriter};
use crate::lcg;
use crate::math;
use crate::prime_tests;

use std::fs::File;
use std::io::Write;

pub struct RsaKeyValues {
    pub n: BigUint,
    pub e: BigUint,
    pub d: BigUint,
}

pub enum RsaKeyType {
    PublicKey,
    PrivateKey,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrimeMethod {
    Naive,
    MillerRabin,
}

impl PrimeMethod {
    pub fn name(&self) -> &'static str {
        match self {
            PrimeMethod::Naive => "Naive",
            PrimeMethod::MillerRabin => "Miller-Rabin",
        }
    }
}

/// Generira praštevilo z izbrano metodo
pub fn generate_prime(lcg: &mut lcg::Lcg, bits: usize, method: PrimeMethod, s: usize) -> BigUint {
    match method {
        PrimeMethod::Naive => prime_tests::naive_prime_generator(lcg, bits),
        PrimeMethod::MillerRabin => prime_tests::miller_rabin_prime_generator(lcg, bits, s),
    }
}

/// Generira RSA ključe iz dveh različnih praštevil p in q, vsako z `prime_bits` biti.
pub fn key_generator(lcg: &mut lcg::Lcg, prime_bits: usize, method: PrimeMethod, s: usize) -> Result<RsaKeyValues, String> {
    let one = BigUint::from(1u32);
    let two = BigUint::from(2u32);

    let mut attempts = 0;
    let (prime1, prime2) = loop {
        let prime1 = generate_prime(lcg, prime_bits, method, s);
        let prime2 = generate_prime(lcg, prime_bits, method, s);

        if prime1 != prime2 { break (prime1, prime2); }

        attempts += 1;
        if attempts > 1000 {
            return Err("Could not find two different primes of this size.".to_string());
        }
    };

    let n: BigUint = &prime1 * &prime2;
    let phi_n: BigUint = (&prime1 - &one) * (&prime2 - &one);

    let (d, e) = loop {
        let mut e_candidate = lcg.random(&two, &(&phi_n - &one)); // (1, phi_n) -> [2, phi_n - 1]
        e_candidate |= &one; // LSb na 1 za liho število
        if math::gcd(&e_candidate, &phi_n) == one && e_candidate < phi_n { // ker nastavimo LSb na 1 mogoče lahko pride do e_candidate == phi_n zato še en check
            let d = math::modular_linear_equation_solver(&e_candidate, &one, &phi_n).ok_or("No modular inverse found for e and phi_n")?;
            break (d, e_candidate);
        }
    };

    return Ok(RsaKeyValues { n, e, d });
}

pub fn save_rsa_key_to_file(rsa_keys: &RsaKeyValues, rsa_key_type: RsaKeyType) -> std::io::Result<&'static str> {
    let (path, exponent) = match rsa_key_type {
        RsaKeyType::PublicKey => ("pubkey.txt", &rsa_keys.e),
        RsaKeyType::PrivateKey => ("privkey.txt", &rsa_keys.d),
    };

    let mut file = File::create(path)?;
    writeln!(file, "{},{}", exponent, rsa_keys.n)?;
    Ok(path)
}

pub fn read_rsa_key_from_file(path: &str) -> Result<(BigUint, BigUint), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("Unable to read key file '{}': {}", path, e))?;
    let (exp, n) = text.trim().split_once(',').ok_or_else(|| format!("'{}' is not a valid key file (expected \"exponent,n\").", path))?;
    let exp: BigUint = exp.trim().parse().map_err(|_| format!("Invalid exponent in '{}'.", path))?;
    let n: BigUint = n.trim().parse().map_err(|_| format!("Invalid modulus in '{}'.", path))?;
    if n.bits() < 2 {
        return Err(format!("Modulus in '{}' is too small.", path));
    }
    Ok((exp, n))
}

/// Zakodira podatke po skupinah (bits(n) - 1) bitov, tako da je vsak blok M < n. Tajnopis zapišemo z bits(n) biti.
/// Format: [1B dolžina končnice][končnica][8B velikost originala][tajnopis]
pub fn encrypt_bytes(input_data: &[u8], ext: &str, e: &BigUint, n: &BigUint) -> Vec<u8> {
    let read_bits_size = (n.bits() - 1) as usize;
    let write_bits_size = n.bits() as usize;

    let mut bit_reader = BitReader::new(input_data);
    let mut bit_writer = BitWriter::new();

    while bit_reader.has_more() {
        let m = bit_reader.read_bits(read_bits_size);
        let c = math::modular_exponetiation(&m, e, n);
        bit_writer.push_bits(&c, write_bits_size);
    }

    let ext_bytes = &ext.as_bytes()[..ext.len().min(255)];

    let mut out = Vec::new();
    out.push(ext_bytes.len() as u8);                                 // 1B za velikost extension tipa
    out.extend_from_slice(ext_bytes);                                 // Extension tip
    out.extend_from_slice(&(input_data.len() as u64).to_be_bytes()); // 8B za velikost originala
    out.extend(bit_writer.finish());
    out
}

/// Vrne (originalni podatki, končnica originalne datoteke)
pub fn decrypt_bytes(input_data: &[u8], d: &BigUint, n: &BigUint) -> Result<(Vec<u8>, String), String> {
    let encrypted_bits_size = n.bits() as usize;
    let write_size = (n.bits() - 1) as usize;

    // Header: [1B ext length][ext][8B original size]
    let bad_file = || "This is not a file produced by this program (bad header).".to_string();
    let ext_len = *input_data.first().ok_or_else(bad_file)? as usize;
    let size_start = 1 + ext_len;
    let data_start = size_start + 8;
    if input_data.len() < data_start { return Err(bad_file()); }

    let ext = String::from_utf8_lossy(&input_data[1..1 + ext_len]).to_string();
    let original_size = u64::from_be_bytes(input_data[size_start..data_start].try_into().unwrap()) as usize;

    let total_chunks = original_size.saturating_mul(8).div_ceil(write_size);
    // Preverimo da je v datoteki dovolj podatkov (sicer ključ sploh ne ustreza datoteki)
    if total_chunks.saturating_mul(encrypted_bits_size) > (input_data.len() - data_start) * 8 {
        return Err("The file is too short for this key / header is corrupt.".to_string());
    }

    let mut bit_reader = BitReader::new(&input_data[data_start..]); // Preskočimo cel header
    let mut bit_writer = BitWriter::new();

    for _ in 0..total_chunks {
        let c = bit_reader.read_bits(encrypted_bits_size);
        let m = math::modular_exponetiation(&c, d, n);
        bit_writer.push_bits(&m, write_size);
    }

    let mut out = bit_writer.finish();
    out.truncate(original_size); // Odstranimo morebitne dodatne bite, ki so nastali zaradi padding-a
    Ok((out, ext))
}

/// Zakodira datoteko in jo shrani v `output_path`.
pub fn encrypt_file(input_path: &str, output_path: &str, rsa_keys_path: &str) -> Result<(), String> {
    let input_data = std::fs::read(input_path).map_err(|e| format!("Unable to read '{}': {}", input_path, e))?;
    let (e, n) = read_rsa_key_from_file(rsa_keys_path)?;

    let ext = std::path::Path::new(input_path).extension().and_then(|e| e.to_str()).unwrap_or("");
    let out = encrypt_bytes(&input_data, ext, &e, &n);

    std::fs::write(output_path, out).map_err(|e| format!("Unable to write '{}': {}", output_path, e))
}

/// Odkodira datoteko. Končnico originalne datoteke dodamo k `output_path`. Vrne dejansko pot izhodne datoteke.
pub fn decrypt_file(input_path: &str, output_path: &str, rsa_keys_path: &str) -> Result<String, String> {
    let input_data = std::fs::read(input_path).map_err(|e| format!("Unable to read '{}': {}", input_path, e))?;
    let (d, n) = read_rsa_key_from_file(rsa_keys_path)?;

    let (out, ext) = decrypt_bytes(&input_data, &d, &n)?;

    let output_path = if ext.is_empty() { output_path.to_string() } else { format!("{}.{}", output_path, ext) };
    std::fs::write(&output_path, out).map_err(|e| format!("Unable to write '{}': {}", output_path, e))?;
    Ok(output_path)
}
