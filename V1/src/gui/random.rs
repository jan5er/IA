use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};
use num_bigint::BigUint;

use super::task::Runner;
use super::util::{copyable, parse_big, Status};
use super::Output;
use crate::bench::{self, Histogram};

pub struct RandomOut {
    pub title: String,
    pub decimal: String,
    pub binary: String,
}

pub struct RandomTab {
    min: String,
    max: String,
    bits: usize,
    pub result: Option<RandomOut>,

    hist_lo: u32,
    hist_hi: u32,
    samples: u64,
    pub histogram: Option<Histogram>,
}

impl Default for RandomTab {
    fn default() -> Self {
        Self {
            min: "1".to_string(),
            max: "100".to_string(),
            bits: 32,
            result: None,
            hist_lo: 1,
            hist_hi: 100,
            samples: 1_000_000,
            histogram: None,
        }
    }
}

impl RandomTab {
    pub fn ui(&mut self, ui: &mut egui::Ui, runner: &mut Runner, status: &mut Status) {
        let busy = runner.busy();

        ui.heading("Random number generator (Super-Duper LCG)");
        ui.label("Rᵢ₊₁ = (69069 · Rᵢ + 1) mod 2³²   -  LCG(2³², 69069, 1, seed)");
        ui.add_space(6.0);

        ui.group(|ui| {
            ui.strong("Random number in the interval [x, y]");
            ui.horizontal(|ui| {
                ui.label("x =");
                ui.add(egui::TextEdit::singleline(&mut self.min).desired_width(160.0));
                ui.label("y =");
                ui.add(egui::TextEdit::singleline(&mut self.max).desired_width(160.0));
                if ui.add_enabled(!busy, egui::Button::new("Generate")).clicked() {
                    match (parse_big(&self.min, "x"), parse_big(&self.max, "y")) {
                        (Ok(min), Ok(max)) if min <= max => {
                            runner.spawn("Generating random number...", move |lcg, _| {
                                let number = lcg.random(&min, &max);
                                Output::Random(RandomOut {
                                    title: format!("RANDOM({}, {})", min, max),
                                    binary: format!("{:b}", number),
                                    decimal: number.to_string(),
                                })
                            });
                        }
                        (Ok(_), Ok(_)) => status.error("x must not be larger than y."),
                        (Err(e), _) | (_, Err(e)) => status.error(e),
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.strong("Random n-bit number");
                ui.add(egui::DragValue::new(&mut self.bits).range(1..=4096).suffix(" bits"));
                if ui.add_enabled(!busy, egui::Button::new("Generate")).on_hover_text("MSB and LSB are set to 1, so the number has exactly n bits and is odd").clicked() {
                    let bits = self.bits;
                    runner.spawn("Generating random number...", move |lcg, _| {
                        let number: BigUint = lcg.generate_candidate(bits);
                        Output::Random(RandomOut {
                            title: format!("Random {}-bit odd number", bits),
                            binary: format!("{:b}", number),
                            decimal: number.to_string(),
                        })
                    });
                }
            });

            if let Some(r) = &self.result {
                ui.separator();
                ui.strong(&r.title);
                copyable(ui, "Decimal:", &r.decimal);
                copyable(ui, "Binary:", &r.binary);
            }
        });

        ui.add_space(10.0);
        ui.heading("Generator analysis: histogram");
        ui.label("Generates many numbers with RANDOM(x, y) and counts how many times each value appears. For a good generator all bars are about equally high.");

        ui.horizontal(|ui| {
            ui.label("x =");
            ui.add(egui::DragValue::new(&mut self.hist_lo).range(0..=u32::MAX - 1));
            ui.label("y =");
            ui.add(egui::DragValue::new(&mut self.hist_hi).range(0..=u32::MAX));
            ui.label("numbers:");
            ui.add(egui::DragValue::new(&mut self.samples).range(1..=1_000_000_000u64).speed(10_000.0));
            if ui.add_enabled(!busy, egui::Button::new("Draw histogram")).clicked() {
                if self.hist_lo > self.hist_hi {
                    status.error("x must not be larger than y.");
                } else {
                    let (lo, hi, samples) = (self.hist_lo, self.hist_hi, self.samples);
                    runner.spawn("Generating numbers...", move |lcg, ctl| Output::Histogram(bench::histogram(lcg, lo, hi, samples, ctl)));
                }
            }
        });
        ui.weak("More than 1000 different values are grouped into 1000 equal bins.");

        if let Some(h) = &self.histogram {
            show_histogram(ui, h);
        }
    }
}

fn show_histogram(ui: &mut egui::Ui, h: &Histogram) {
    let bins = h.counts.len();
    let unit = if bins as u64 == h.hi as u64 - h.lo as u64 + 1 { "value" } else { "bin" };

    // Sredina predala v enotah vrednosti
    let mut bars = Vec::with_capacity(bins);
    let mut expected_line = Vec::with_capacity(bins);
    let mut first_offset = 0u64;
    for i in 0..bins {
        let width = h.values_per_bin[i] as f64;
        let center = h.lo as f64 + first_offset as f64 + (width - 1.0) / 2.0;
        bars.push(Bar::new(center, h.counts[i] as f64).width(width));
        expected_line.push([center, h.expected[i]]);
        first_offset += h.values_per_bin[i];
    }

    let max = *h.counts.iter().max().unwrap();
    let min = *h.counts.iter().min().unwrap();
    let mean_expected = h.samples as f64 / bins as f64;
    let dof = (bins - 1).max(1) as f64;

    ui.add_space(4.0);
    Plot::new("histogram")
        .height(320.0)
        .allow_scroll(false)
        .include_y(0.0)
        .x_axis_label("generated number")
        .y_axis_label("occurrences")
        .show(ui, |plot_ui| {
            plot_ui.bar_chart(BarChart::new(format!("Occurrences per {}", unit), bars));
            plot_ui.line(Line::new("Expected (uniform)", PlotPoints::from(expected_line)).width(2.0));
        });

    ui.label(format!(
        "{} numbers in [{}, {}], {} {}s. Expected per {}: {:.1}. Smallest count: {} ({:+.2} %), largest: {} ({:+.2} %).",
        h.samples, h.lo, h.hi, bins, unit, unit, mean_expected,
        min, (min as f64 / mean_expected - 1.0) * 100.0, max, (max as f64 / mean_expected - 1.0) * 100.0
    ));
    ui.label(format!(
        "Chi-squared test: χ² = {:.1} with {} degrees of freedom, χ²/dof = {:.2} (a uniform distribution gives about 1; much larger values mean a biased generator).",
        h.chi_squared, bins - 1, h.chi_squared / dof
    ));
}
