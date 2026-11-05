//! Prvotni konzolni meni (zaženi z `cargo run -- --cli`)

use num_bigint::BigUint;
use std::io;

use crate::lcg;
use crate::prime_tests;
use crate::rsa::{self, PrimeMethod, RsaKeyType};

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

fn read_line() -> String {
    let mut line = String::new();
    io::stdin().read_line(&mut line).expect("Failed to read line");
    line.trim().to_string()
}

pub fn run(seed: u32) {
    let mut lcg = lcg::Lcg::new(seed);

    loop {
        menu();

        let Ok(choice) = read_line().parse::<u8>() else {
            println!("Please type a number!");
            continue;
        };

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
                match rsa::key_generator(&mut lcg, 10, PrimeMethod::MillerRabin, 10) {
                    Ok(rsa_keys) => {
                        println!("RSA keys generated:");
                        println!("n: {}", rsa_keys.n);
                        println!("e: {}", rsa_keys.e);
                        println!("d: {}", rsa_keys.d);
                        println!("Public Key: (e, n) = ({}, {})", rsa_keys.e, rsa_keys.n);
                        println!("Private Key: (d, n) = ({}, {})", rsa_keys.d, rsa_keys.n);
                        let _ = rsa::save_rsa_key_to_file(&rsa_keys, RsaKeyType::PublicKey);
                        let _ = rsa::save_rsa_key_to_file(&rsa_keys, RsaKeyType::PrivateKey);
                    }
                    Err(e) => println!("{}", e),
                }
            }
            5 => {
                println!("Enter the path of the file to encrypt:");
                let file_path = read_line();
                match rsa::encrypt_file(&file_path, "encrypted.bin", "pubkey.txt") {
                    Ok(()) => println!("Saved to encrypted.bin"),
                    Err(e) => println!("{}", e),
                }
            }
            6 => {
                println!("Enter the path of the file to decrypt:");
                let file_path = read_line();
                match rsa::decrypt_file(&file_path, "decrypted", "privkey.txt") {
                    Ok(path) => println!("Saved to {}", path),
                    Err(e) => println!("{}", e),
                }
            }
            _ => {
                println!("Invalid choice. Please try again.");
            }
        }
    }
}
