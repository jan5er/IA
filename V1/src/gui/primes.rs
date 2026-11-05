use std::time::Instant;

use super::task::Runner;
use super::util::{copyable, format_micros, parse_big, Status};
use super::Output;
use crate::prime_tests::{self, MillerRabinResult};
use crate::rsa::{self, PrimeMethod, NAIVE_MAX_BITS};

/// Največje število bitov, pri katerem naivni test praštevilskosti še teče v sprejemljivem času
const NAIVE_TEST_MAX_BITS: u64 = 40;

pub struct PrimeOut {
    method: PrimeMethod,
    s: usize,
    bits: usize,
    decimal: String,
    binary: String,
    micros: f64,
}

pub struct PrimeTestOut {
    number: String,
    s: usize,
    /// (je praštevilo, čas v µs) ali razlog, zakaj ni bilo testirano
    naive: Result<(bool, f64), String>,
    miller_rabin: (&'static str, f64),
}

pub struct PrimesTab {
    bits: usize,
    method: PrimeMethod,
    s: usize,
    pub generated: Option<PrimeOut>,

    test_input: String,
    pub tested: Option<PrimeTestOut>,
}

impl Default for PrimesTab {
    fn default() -> Self {
        Self {
            bits: 16,
            method: PrimeMethod::MillerRabin,
            s: 10,
            generated: None,
            test_input: "2147483647".to_string(),
            tested: None,
        }
    }
}

impl PrimesTab {
    pub fn ui(&mut self, ui: &mut egui::Ui, runner: &mut Runner, status: &mut Status) {
        let busy = runner.busy();

        ui.heading("Prime numbers");
        ui.add_space(6.0);

        ui.group(|ui| {
            ui.strong("Generate an n-bit prime");
            ui.horizontal(|ui| {
                ui.label("Method:");
                ui.radio_value(&mut self.method, PrimeMethod::Naive, "Naive");
                ui.radio_value(&mut self.method, PrimeMethod::MillerRabin, "Miller-Rabin");
            });

            let max_bits = if self.method == PrimeMethod::Naive { NAIVE_MAX_BITS } else { 4096 };
            self.bits = self.bits.clamp(2, max_bits);
            ui.horizontal(|ui| {
                ui.label("Size:");
                ui.add(egui::DragValue::new(&mut self.bits).range(2..=max_bits).suffix(" bits"));
                ui.weak(format!("(an n-bit number lies between 1000...0 and 1111...1, here 2^{} and 2^{} - 1)", self.bits - 1, self.bits));
            });
            if self.method == PrimeMethod::MillerRabin {
                ui.horizontal(|ui| {
                    ui.label("Parameter s:");
                    ui.add(egui::DragValue::new(&mut self.s).range(1..=200));
                    ui.weak("(number of Miller-Rabin rounds; the error probability is at most 4^-s)");
                });
            } else {
                ui.weak(format!("The naive method is limited to {} bits.", NAIVE_MAX_BITS));
            }

            if ui.add_enabled(!busy, egui::Button::new("Generate prime")).clicked() {
                let (method, bits, s) = (self.method, self.bits, self.s);
                runner.spawn("Searching for a prime...", move |lcg, _| {
                    let start = Instant::now();
                    let prime = rsa::generate_prime(lcg, bits, method, s);
                    let micros = start.elapsed().as_secs_f64() * 1e6;
                    Output::Prime(PrimeOut { method, s, bits, decimal: prime.to_string(), binary: format!("{:b}", prime), micros })
                });
            }

            if let Some(p) = &self.generated {
                ui.separator();
                let how = match p.method {
                    PrimeMethod::Naive => "naive method".to_string(),
                    PrimeMethod::MillerRabin => format!("Miller-Rabin, s = {}", p.s),
                };
                ui.strong(format!("{}-bit prime ({}) found in {}", p.bits, how, format_micros(p.micros)));
                copyable(ui, "Decimal:", &p.decimal);
                copyable(ui, "Binary:", &p.binary);
            }
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.strong("Is this number prime?");
            ui.horizontal(|ui| {
                ui.label("Number:");
                ui.add(egui::TextEdit::singleline(&mut self.test_input).desired_width(300.0));
                ui.label("s =");
                ui.add(egui::DragValue::new(&mut self.s).range(1..=200)).on_hover_text("Miller-Rabin parameter");
                if ui.add_enabled(!busy, egui::Button::new("Test with both methods")).clicked() {
                    match parse_big(&self.test_input, "The number") {
                        Ok(number) => {
                            let s = self.s;
                            runner.spawn("Testing...", move |lcg, _| {
                                let naive = if number.bits() > NAIVE_TEST_MAX_BITS {
                                    Err(format!("skipped: numbers over {} bits would take too long", NAIVE_TEST_MAX_BITS))
                                } else {
                                    let start = Instant::now();
                                    let result = prime_tests::naive_is_prime(&number);
                                    Ok((result, start.elapsed().as_secs_f64() * 1e6))
                                };

                                let start = Instant::now();
                                let result = match prime_tests::miller_rabin_test(lcg, number.clone(), s) {
                                    MillerRabinResult::Prime => "prime",
                                    MillerRabinResult::ProbablyPrime => "probably prime",
                                    MillerRabinResult::CompositeNumber => "composite",
                                };
                                let miller_rabin = (result, start.elapsed().as_secs_f64() * 1e6);

                                Output::PrimeTest(PrimeTestOut { number: number.to_string(), s, naive, miller_rabin })
                            });
                        }
                        Err(e) => status.error(e),
                    }
                }
            });

            if let Some(t) = &self.tested {
                ui.separator();
                ui.strong(format!("Result for {}", t.number));
                egui::Grid::new("prime_test_results").num_columns(3).spacing([20.0, 4.0]).show(ui, |ui| {
                    ui.label("Naive method:");
                    match &t.naive {
                        Ok((is_prime, micros)) => {
                            ui.label(if *is_prime { "prime" } else { "composite" });
                            ui.label(format_micros(*micros));
                        }
                        Err(reason) => {
                            ui.weak(reason);
                            ui.label("");
                        }
                    }
                    ui.end_row();

                    ui.label(format!("Miller-Rabin (s = {}):", t.s));
                    ui.label(t.miller_rabin.0);
                    ui.label(format_micros(t.miller_rabin.1));
                    ui.end_row();
                });
                ui.weak("\"Probably prime\" means no witness of compositeness was found in s rounds.");
            }
        });
    }
}
