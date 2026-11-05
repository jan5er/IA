use std::io;
use num_bigint::BigUint;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bits::test_bit_writer;

mod lcg;    
mod prime_tests;
mod math;
mod rsa;
mod bits;

fn get_time_seed() -> u32 {
    return SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u32;
}

fn menu() {
    println!("--------[ MENU ]--------");
    println!("[1] Generate random number");
    println!("[2] Generate prime number");
    println!("[3] Generate prime number (Miller-Rabin)");
    println!("[4] Generate RSA keys");
    println!("[5] Encrypt a file");
    println!("[6] Decrypt a file");
    println!("------------------------");
}

fn main() {
    // let seed: u32 = get_time_seed();
    let seed: u32 = 0;
    let mut lcg = lcg::Lcg::new(seed);

    loop {
        let mut choice = String::new();
        menu();

        io::stdin().read_line(&mut choice).expect("Failed to read line");

        let choice: u8 = choice.trim().parse().expect("Please type a number!");

        match choice {
            1 => {
                println!("Generating random number...");
                let random_number = lcg.generate_candidate(50);
                let random: BigUint = lcg.random(&BigUint::from(10u32), &BigUint::from(100u32));
                println!("Random number: {}", random_number);
                println!("Random number in binary: {:b}", random_number);
                println!("Random random: {}", random);
            }
            2 => {
                println!("Generating prime number...");
                let prime = prime_tests::naive_prime_generator(&mut lcg, 50);
                println!("Prime number: {}", prime);
                println!("Prime number in binary: {:b}", prime);
            }
            3 => {
                println!("Generating prime number (Miller-Rabin)...");
                let prime = prime_tests::miller_rabin_prime_generator(&mut lcg, 1024, 10);
                println!("Prime number: {}", prime);
                println!("Prime number in binary: {:b}", prime);
            }
            4 => {
                println!("Generating RSA keys...");
                let rsa_keys = rsa::key_generator(&mut lcg);
                println!("RSA keys generated:");
                println!("n: {}", rsa_keys.n);
                println!("e: {}", rsa_keys.e);
                println!("d: {}", rsa_keys.d);
                println!("Public Key: (e, n) = ({}, {})", rsa_keys.e, rsa_keys.n);
                println!("Private Key: (d, n) = ({}, {})", rsa_keys.d, rsa_keys.n);
                rsa::save_rsa_key_to_file(&rsa_keys, rsa::RsaKeyType::PublicKey);
                rsa::save_rsa_key_to_file(&rsa_keys, rsa::RsaKeyType::PrivateKey);
            }
            5 => {
                println!("Encrypting a file...");
                let mut file_path = String::new();
                println!("Enter the path of the file to encrypt:");
                io::stdin().read_line(&mut file_path).expect("Failed to read line");
                let file_path = file_path.trim();
                rsa::encrypt_file(file_path, "encrypted.bin", "pubkey.txt");
            }
            6 => {
                println!("Decrypting a file...");
                let mut file_path = String::new();
                println!("Enter the path of the file to decrypt:");
                io::stdin().read_line(&mut file_path).expect("Failed to read line");
                let file_path = file_path.trim();
                rsa::decrypt_file(file_path, "decrypted", "privkey.txt");
            }
            _ => {
                println!("Invalid choice. Please try again.");
            }
        }
    }


}   
