use egui_plot::{HoverPosition, Legend, Line, Plot, PlotPoints, Points};

use super::task::Runner;
use super::util::{format_micros, Status};
use super::Output;
use crate::bench::{self, ChartData};
use crate::rsa::NAIVE_MAX_BITS;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChartKind {
    PrimeGeneration,
    MillerRabinS,
    KeyGeneration,
    Crypt,
}

pub struct ChartsTab {
    log_scale: bool,

    prime_min: usize,
    prime_max: usize,
    prime_reps: usize,
    prime_s: usize,
    prime_chart: Option<ChartData>,

    s_bits: usize,
    s_max: usize,
    s_reps: usize,
    s_chart: Option<ChartData>,

    key_min: usize,
    key_max: usize,
    key_reps: usize,
    key_s: usize,
    key_charts: Vec<ChartData>,

    crypt_min: usize,
    crypt_max: usize,
    crypt_reps: usize,
    crypt_chart: Option<ChartData>,
}

impl Default for ChartsTab {
    fn default() -> Self {
        Self {
            log_scale: false,
            prime_min: 4, prime_max: 32, prime_reps: 20, prime_s: 10, prime_chart: None,
            s_bits: 32, s_max: 20, s_reps: 50, s_chart: None,
            key_min: 3, key_max: 15, key_reps: 200, key_s: 10, key_charts: Vec::new(),
            crypt_min: 3, crypt_max: 15, crypt_reps: 200, crypt_chart: None,
        }
    }
}

impl ChartsTab {
    pub fn set(&mut self, kind: ChartKind, mut charts: Vec<ChartData>) {
        match kind {
            ChartKind::PrimeGeneration => self.prime_chart = charts.pop(),
            ChartKind::MillerRabinS => self.s_chart = charts.pop(),
            ChartKind::KeyGeneration => self.key_charts = charts,
            ChartKind::Crypt => self.crypt_chart = charts.pop(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, runner: &mut Runner, status: &mut Status) {
        let busy = runner.busy();

        ui.heading("Time complexity");
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.log_scale, "Logarithmic time axis").on_hover_text("Naive and Miller-Rabin differ by orders of magnitude, a log axis makes both visible");
            ui.weak("Each point is the average over several repetitions, each with a different random number. Run the release build for meaningful numbers.");
        });
        ui.add_space(6.0);

        // ---------------------------------------------------------------- 1.1: naivna proti MR
        egui::CollapsingHeader::new("Prime generation: naive vs Miller-Rabin (assignment 1.1)").default_open(true).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("n from");
                ui.add(egui::DragValue::new(&mut self.prime_min).range(2..=NAIVE_MAX_BITS));
                ui.label("to");
                ui.add(egui::DragValue::new(&mut self.prime_max).range(2..=NAIVE_MAX_BITS));
                ui.label("bits   repetitions:");
                ui.add(egui::DragValue::new(&mut self.prime_reps).range(1..=10_000));
                ui.label("Miller-Rabin s:");
                ui.add(egui::DragValue::new(&mut self.prime_s).range(1..=200));
                if ui.add_enabled(!busy, egui::Button::new("Run")).clicked() {
                    let (min, max, reps, s) = (self.prime_min, self.prime_max.max(self.prime_min), self.prime_reps, self.prime_s);
                    runner.spawn("Measuring prime generation...", move |lcg, ctl| {
                        Output::Charts(ChartKind::PrimeGeneration, vec![bench::prime_generation_time(lcg, min, max, reps, s, ctl)])
                    });
                }
            });
            if let Some(chart) = &self.prime_chart {
                show_chart(ui, "chart_primes", chart, self.log_scale);
            }
        });

        // ---------------------------------------------------------------- 1.1: MR glede na s
        egui::CollapsingHeader::new("Miller-Rabin: time vs parameter s (assignment 1.1)").default_open(true).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Prime size:");
                ui.add(egui::DragValue::new(&mut self.s_bits).range(2..=4096).suffix(" bits"));
                ui.label("s from 1 to");
                ui.add(egui::DragValue::new(&mut self.s_max).range(1..=200));
                ui.label("repetitions:");
                ui.add(egui::DragValue::new(&mut self.s_reps).range(1..=100_000));
                if ui.add_enabled(!busy, egui::Button::new("Run")).clicked() {
                    let (bits, s_max, reps) = (self.s_bits, self.s_max, self.s_reps);
                    runner.spawn("Measuring Miller-Rabin vs s...", move |lcg, ctl| {
                        Output::Charts(ChartKind::MillerRabinS, vec![bench::miller_rabin_time_vs_s(lcg, bits, 1, s_max, reps, ctl)])
                    });
                }
            });
            if let Some(chart) = &self.s_chart {
                show_chart(ui, "chart_s", chart, self.log_scale);
            }
        });

        // ---------------------------------------------------------------- 1.2: tvorba ključev
        egui::CollapsingHeader::new("RSA key generation time (assignment 1.2)").default_open(true).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Bits of p and q from");
                ui.add(egui::DragValue::new(&mut self.key_min).range(3..=1024));
                ui.label("to");
                ui.add(egui::DragValue::new(&mut self.key_max).range(3..=1024));
                ui.label("repetitions:");
                ui.add(egui::DragValue::new(&mut self.key_reps).range(1..=100_000));
                ui.label("Miller-Rabin s:");
                ui.add(egui::DragValue::new(&mut self.key_s).range(1..=200));
                if ui.add_enabled(!busy, egui::Button::new("Run")).clicked() {
                    let (min, max, reps, s) = (self.key_min, self.key_max.max(self.key_min), self.key_reps, self.key_s);
                    runner.spawn("Measuring key generation...", move |lcg, ctl| {
                        Output::Charts(ChartKind::KeyGeneration, bench::keygen_time(lcg, min, max, reps, s, ctl))
                    });
                }
            });
            ui.weak(format!("The naive method only supports up to {} bits.", NAIVE_MAX_BITS));
            for (i, chart) in self.key_charts.iter().enumerate() {
                show_chart(ui, &format!("chart_keys_{}", i), chart, self.log_scale);
            }
        });

        // ---------------------------------------------------------------- 1.2: kodiranje / dekodiranje
        egui::CollapsingHeader::new("Encryption / decryption time (assignment 1.2)").default_open(true).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Bits of p and q from");
                ui.add(egui::DragValue::new(&mut self.crypt_min).range(3..=1024));
                ui.label("to");
                ui.add(egui::DragValue::new(&mut self.crypt_max).range(3..=1024));
                ui.label("repetitions:");
                ui.add(egui::DragValue::new(&mut self.crypt_reps).range(1..=100_000));
                if ui.add_enabled(!busy, egui::Button::new("Run")).clicked() {
                    let (min, max, reps, s) = (self.crypt_min, self.crypt_max.max(self.crypt_min), self.crypt_reps, self.key_s);
                    runner.spawn("Measuring encryption / decryption...", move |lcg, ctl| {
                        Output::Charts(ChartKind::Crypt, vec![bench::crypt_time(lcg, min, max, reps, s, ctl)])
                    });
                }
            });
            if let Some(chart) = &self.crypt_chart {
                show_chart(ui, "chart_crypt", chart, self.log_scale);
            }
        });

        let _ = status;
    }
}

fn show_chart(ui: &mut egui::Ui, id: &str, chart: &ChartData, log_scale: bool) {
    ui.add_space(4.0);
    ui.strong(&chart.title);

    // Pri logaritemski osi narišemo log10(čas), oznake osi pa nazaj pretvorimo v čas
    let transform = move |y: f64| if log_scale { y.max(1e-3).log10() } else { y };
    let untransform = move |v: f64| if log_scale { 10f64.powf(v) } else { v };

    Plot::new(id.to_string())
        .height(300.0)
        .legend(Legend::default())
        .allow_scroll(false)
        .x_axis_label(chart.x_label.clone())
        .y_axis_label(if log_scale { "time (log scale)".to_string() } else { chart.y_label.clone() })
        .y_axis_formatter(move |mark, _| format_micros(untransform(mark.value)))
        .label_formatter(move |hover| match hover {
            HoverPosition::NearDataPoint { plot_name, position, .. } => Some(format!(
                "{}
x = {:.1}
time = {}", plot_name, position.x, format_micros(untransform(position.y))
            )),
            HoverPosition::Elsewhere { .. } => None,
        })
        .show(ui, |plot_ui| {
            for series in &chart.series {
                let points: Vec<[f64; 2]> = series.points.iter().map(|p| [p[0], transform(p[1])]).collect();
                plot_ui.line(Line::new(series.name.clone(), PlotPoints::from(points.clone())).width(2.0));
                plot_ui.points(Points::new(series.name.clone(), PlotPoints::from(points)).radius(3.5));
            }
        });

    if !chart.note.is_empty() {
        ui.weak(&chart.note);
    }
}
