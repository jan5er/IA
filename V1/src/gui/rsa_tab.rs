use std::time::Instant;

use super::task::Runner;
use super::util::{copyable, format_micros, parse_big, Status};
use super::Output;
use crate::math;
use crate::rsa::{self, PrimeMethod, RsaKeyType, NAIVE_MAX_BITS};

pub struct KeysOut {
    method: PrimeMethod,
    prime_bits: usize,
    n: String,
    e: String,
    d: String,
    n_bits: u64,
    micros: f64,
    saved: Result<(), String>,
}

pub struct RoundTripOut {
    m: String,
    c: String,
    decrypted: String,
}

pub struct RsaTab {
    prime_bits: usize,
    method: PrimeMethod,
    s: usize,
    pub keys: Option<KeysOut>,

    pub_path: String,
    priv_path: String,

    number: String,
    pub round_trip: Option<RoundTripOut>,

    encrypt_in: String,
    encrypt_out: String,
    decrypt_in: String,
    decrypt_out: String,
}

impl Default for RsaTab {
    fn default() -> Self {
        Self {
            prime_bits: 15,
            method: PrimeMethod::MillerRabin,
            s: 10,
            keys: None,
            pub_path: "pubkey.txt".to_string(),
            priv_path: "privkey.txt".to_string(),
            number: "12345".to_string(),
            round_trip: None,
            encrypt_in: String::new(),
            encrypt_out: "enc.bin".to_string(),
            decrypt_in: "enc.bin".to_string(),
            decrypt_out: "msg".to_string(),
        }
    }
}

/// Polje za pot do datoteke z gumbom "Browse..."
fn path_row(ui: &mut egui::Ui, label: &str, path: &mut String, pick: impl FnOnce() -> Option<std::path::PathBuf>) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::TextEdit::singleline(path).desired_width(380.0));
        if ui.button("Browse...").clicked() {
            if let Some(chosen) = pick() {
                *path = chosen.display().to_string();
            }
        }
    });
}

impl RsaTab {
    pub fn ui(&mut self, ui: &mut egui::Ui, runner: &mut Runner, status: &mut Status) {
        let busy = runner.busy();

        ui.heading("RSA");
        ui.add_space(6.0);

        // ---------------------------------------------------------------- Ključi
        ui.group(|ui| {
            ui.strong("1. Generate a key pair");
            ui.horizontal(|ui| {
                ui.label("Prime generation:");
                ui.radio_value(&mut self.method, PrimeMethod::Naive, "Naive");
                ui.radio_value(&mut self.method, PrimeMethod::MillerRabin, "Miller-Rabin");
            });

            let max_bits = if self.method == PrimeMethod::Naive { NAIVE_MAX_BITS } else { 2048 };
            self.prime_bits = self.prime_bits.clamp(3, max_bits);
            ui.horizontal(|ui| {
                ui.label("Bits of p and q:");
                ui.add(egui::DragValue::new(&mut self.prime_bits).range(3..=max_bits));
                ui.weak(format!("(n = p·q will have about {} bits; the basic version needs up to 15, Miller-Rabin allows up to 2048)", self.prime_bits * 2));
            });
            if self.method == PrimeMethod::MillerRabin {
                ui.horizontal(|ui| {
                    ui.label("Miller-Rabin parameter s:");
                    ui.add(egui::DragValue::new(&mut self.s).range(1..=200));
                });
            }

            if ui.add_enabled(!busy, egui::Button::new("Generate keys and save")).on_hover_text("Writes pubkey.txt and privkey.txt into the working directory").clicked() {
                let (method, prime_bits, s) = (self.method, self.prime_bits, self.s);
                runner.spawn("Generating RSA keys...", move |lcg, _| {
                    let start = Instant::now();
                    let keys = match rsa::key_generator(lcg, prime_bits, method, s) {
                        Ok(keys) => keys,
                        Err(e) => return Output::Error(e),
                    };
                    let micros = start.elapsed().as_secs_f64() * 1e6;

                    let saved = rsa::save_rsa_key_to_file(&keys, RsaKeyType::PublicKey)
                        .and_then(|_| rsa::save_rsa_key_to_file(&keys, RsaKeyType::PrivateKey))
                        .map(|_| ())
                        .map_err(|e| e.to_string());

                    Output::Keys(KeysOut {
                        method, prime_bits, micros, saved,
                        n_bits: keys.n.bits(),
                        n: keys.n.to_string(), e: keys.e.to_string(), d: keys.d.to_string(),
                    })
                });
            }

            if let Some(k) = &self.keys {
                ui.separator();
                ui.strong(format!("Keys generated in {} ({}, p and q: {} bits, n: {} bits)", format_micros(k.micros), k.method.name(), k.prime_bits, k.n_bits));
                copyable(ui, "n = p·q:", &k.n);
                copyable(ui, "e (public exponent):", &k.e);
                copyable(ui, "d (private exponent):", &k.d);
                match &k.saved {
                    Ok(()) => { ui.label("Public key (e, n) saved to pubkey.txt, private key (d, n) saved to privkey.txt."); }
                    Err(e) => { ui.colored_label(ui.visuals().error_fg_color, format!("Could not save the key files: {}", e)); }
                }
            }
        });

        // ---------------------------------------------------------------- Ključi iz datotek
        ui.add_space(10.0);
        ui.group(|ui| {
            ui.strong("Key files used below");
            path_row(ui, "Public key:   ", &mut self.pub_path, || rsa_file_dialog().pick_file());
            path_row(ui, "Private key:  ", &mut self.priv_path, || rsa_file_dialog().pick_file());
        });

        // ---------------------------------------------------------------- Število
        ui.add_space(10.0);
        ui.group(|ui| {
            ui.strong("2. Encrypt and decrypt a number");
            ui.horizontal(|ui| {
                ui.label("M =");
                ui.add(egui::TextEdit::singleline(&mut self.number).desired_width(300.0));
                if ui.add_enabled(!busy, egui::Button::new("C = M^e mod n, then M = C^d mod n")).clicked() {
                    match parse_big(&self.number, "M") {
                        Ok(m) => {
                            let (pub_path, priv_path) = (self.pub_path.clone(), self.priv_path.clone());
                            runner.spawn("Encrypting...", move |_, _| {
                                let (e, n) = match rsa::read_rsa_key_from_file(&pub_path) { Ok(k) => k, Err(e) => return Output::Error(e) };
                                let (d, n2) = match rsa::read_rsa_key_from_file(&priv_path) { Ok(k) => k, Err(e) => return Output::Error(e) };
                                if n != n2 { return Output::Error("The public and private key have different n - they are not a pair.".to_string()); }
                                if m >= n { return Output::Error(format!("M must be smaller than n = {}.", n)); }

                                let c = math::modular_exponetiation(&m, &e, &n);
                                let decrypted = math::modular_exponetiation(&c, &d, &n);
                                Output::RoundTrip(RoundTripOut { m: m.to_string(), c: c.to_string(), decrypted: decrypted.to_string() })
                            });
                        }
                        Err(e) => status.error(e),
                    }
                }
            });

            if let Some(r) = &self.round_trip {
                ui.separator();
                copyable(ui, "M (message):", &r.m);
                copyable(ui, "C (ciphertext):", &r.c);
                copyable(ui, "Decrypted:", &r.decrypted);
                if r.decrypted == r.m {
                    ui.colored_label(egui::Color32::from_rgb(60, 160, 80), "The decrypted number matches the message.");
                } else {
                    ui.colored_label(ui.visuals().error_fg_color, "The decrypted number does NOT match - the keys are not a pair.");
                }
            }
        });

        // ---------------------------------------------------------------- Datoteke
        ui.add_space(10.0);
        ui.group(|ui| {
            ui.strong("3. Encrypt a file (any type) with the public key");
            path_row(ui, "File:  ", &mut self.encrypt_in, || rfd::FileDialog::new().pick_file());
            path_row(ui, "Save as:", &mut self.encrypt_out, || rfd::FileDialog::new().set_file_name("enc.bin").save_file());
            if ui.add_enabled(!busy, egui::Button::new("Encrypt file")).clicked() {
                let (input, output, key) = (self.encrypt_in.clone(), self.encrypt_out.clone(), self.pub_path.clone());
                if input.trim().is_empty() {
                    status.error("Choose a file to encrypt first.");
                } else {
                    runner.spawn("Encrypting file...", move |_, _| {
                        let start = Instant::now();
                        match rsa::encrypt_file(&input, &output, &key) {
                            Ok(()) => {
                                let before = std::fs::metadata(&input).map(|m| m.len()).unwrap_or(0);
                                let after = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
                                Output::FileDone(format!(
                                    "Encrypted {} bytes into {} ({} bytes, {:.2}x the original) in {}.",
                                    before, output, after, after as f64 / before.max(1) as f64,
                                    format_micros(start.elapsed().as_secs_f64() * 1e6)
                                ))
                            }
                            Err(e) => Output::Error(e),
                        }
                    });
                }
            }
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.strong("4. Decrypt a file with the private key");
            path_row(ui, "File:  ", &mut self.decrypt_in, || rfd::FileDialog::new().pick_file());
            path_row(ui, "Save as:", &mut self.decrypt_out, || rfd::FileDialog::new().set_file_name("msg").save_file());
            ui.weak("The original file extension is stored inside the encrypted file and appended to the name (msg -> msg.pdf).");
            if ui.add_enabled(!busy, egui::Button::new("Decrypt file")).clicked() {
                let (input, output, key) = (self.decrypt_in.clone(), self.decrypt_out.clone(), self.priv_path.clone());
                if input.trim().is_empty() {
                    status.error("Choose a file to decrypt first.");
                } else {
                    runner.spawn("Decrypting file...", move |_, _| {
                        let start = Instant::now();
                        match rsa::decrypt_file(&input, &output, &key) {
                            Ok(written) => Output::FileDone(format!(
                                "Decrypted into {} in {}.",
                                written, format_micros(start.elapsed().as_secs_f64() * 1e6)
                            )),
                            Err(e) => Output::Error(e),
                        }
                    });
                }
            }
        });
    }
}

fn rsa_file_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new().add_filter("Key file", &["txt"]).add_filter("All files", &["*"])
}
