//! Grafični vmesnik (egui) za nalogi 1.1 (praštevila) in 1.2 (RSA).

pub mod task;
mod charts;
mod primes;
mod random;
mod rsa_tab;
mod util;

use std::time::{SystemTime, UNIX_EPOCH};

use task::Runner;
use util::Status;

/// Rezultat opravila, ki je teklo v ozadju; `App::update` ga razporedi v pravi zavihek.
pub enum Output {
    Error(String),
    Random(random::RandomOut),
    Histogram(crate::bench::Histogram),
    Prime(primes::PrimeOut),
    PrimeTest(primes::PrimeTestOut),
    Keys(rsa_tab::KeysOut),
    RoundTrip(rsa_tab::RoundTripOut),
    FileDone(String),
    Charts(charts::ChartKind, Vec<crate::bench::ChartData>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Random,
    Primes,
    Rsa,
    Charts,
}

pub struct App {
    runner: Runner,
    tab: Tab,
    seed_text: String,
    status: Status,

    random: random::RandomTab,
    primes: primes::PrimesTab,
    rsa: rsa_tab::RsaTab,
    charts: charts::ChartsTab,
}

fn time_seed() -> u32 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u32
}

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 780.0]).with_min_inner_size([700.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Izbrani algoritmi - Primes & RSA",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let seed = time_seed();
        Self {
            runner: Runner::new(cc.egui_ctx.clone(), seed),
            tab: Tab::Random,
            seed_text: seed.to_string(),
            status: Status::default(),
            random: Default::default(),
            primes: Default::default(),
            rsa: Default::default(),
            charts: Default::default(),
        }
    }

    fn handle_output(&mut self, output: Output) {
        match output {
            Output::Error(msg) => self.status.error(msg),
            Output::Random(r) => { self.random.result = Some(r); self.status.clear(); }
            Output::Histogram(h) => { self.random.histogram = Some(h); self.status.clear(); }
            Output::Prime(p) => { self.primes.generated = Some(p); self.status.clear(); }
            Output::PrimeTest(t) => { self.primes.tested = Some(t); self.status.clear(); }
            Output::Keys(k) => { self.rsa.keys = Some(k); self.status.clear(); }
            Output::RoundTrip(r) => { self.rsa.round_trip = Some(r); self.status.clear(); }
            Output::FileDone(msg) => self.status.info(msg),
            Output::Charts(kind, charts) => { self.charts.set(kind, charts); self.status.clear(); }
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let busy = self.runner.busy();

        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Random, "Random numbers (LCG)");
            ui.selectable_value(&mut self.tab, Tab::Primes, "Primes");
            ui.selectable_value(&mut self.tab, Tab::Rsa, "RSA");
            ui.selectable_value(&mut self.tab, Tab::Charts, "Timing charts");

            ui.separator();
            ui.label("LCG seed:");
            ui.add_enabled(!busy, egui::TextEdit::singleline(&mut self.seed_text).desired_width(90.0));
            if ui.add_enabled(!busy, egui::Button::new("Apply")).on_hover_text("Restart the generator from this seed (same seed = same numbers)").clicked() {
                match self.seed_text.trim().parse::<u32>() {
                    Ok(seed) => { self.runner.reseed(seed); self.status.info(format!("LCG restarted with seed {}.", seed)); }
                    Err(_) => self.status.error("The seed must be a number between 0 and 4294967295."),
                }
            }
            if ui.add_enabled(!busy, egui::Button::new("Time seed")).clicked() {
                let seed = time_seed();
                self.seed_text = seed.to_string();
                self.runner.reseed(seed);
                self.status.info(format!("LCG restarted with seed {}.", seed));
            }
        });
    }

    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(label) = self.runner.label() {
                ui.spinner();
                ui.label(label);
                match self.runner.progress() {
                    Some(p) => { ui.add(egui::ProgressBar::new(p).desired_width(220.0).show_percentage()); }
                    None => {}
                }
                if ui.button("Cancel").clicked() {
                    self.runner.cancel();
                }
            } else if let Some((is_error, text)) = self.status.get() {
                let color = if is_error { ui.visuals().error_fg_color } else { ui.visuals().text_color() };
                ui.colored_label(color, text);
            } else {
                ui.weak("Ready.");
            }

            if cfg!(debug_assertions) {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(ui.visuals().warn_fg_color, "Debug build: for meaningful timings use `cargo run --release`");
                });
            }
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(output) = self.runner.poll() {
            self.handle_output(output);
        }

        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(4.0);
            self.top_bar(ui);
            ui.add_space(4.0);
        });
        egui::Panel::bottom("bottom").show(ui, |ui| {
            ui.add_space(4.0);
            self.bottom_bar(ui);
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.tab {
                Tab::Random => self.random.ui(ui, &mut self.runner, &mut self.status),
                Tab::Primes => self.primes.ui(ui, &mut self.runner, &mut self.status),
                Tab::Rsa => self.rsa.ui(ui, &mut self.runner, &mut self.status),
                Tab::Charts => self.charts.ui(ui, &mut self.runner, &mut self.status),
            });
        });
    }
}
