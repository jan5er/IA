//! Opravila tečejo v ločeni niti, da se okno med računanjem (npr. generiranje 2048-bitnih praštevil) ne zamrzne.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};

use crate::lcg::Lcg;
use super::Output;

enum Msg {
    Progress(f32),
    Done(Output),
}

/// Ročica, ki jo dobi opravilo (napredek, preklic)
pub struct TaskCtl {
    tx: Sender<Msg>,
    cancel: Arc<AtomicBool>,
    ctx: egui::Context,
}

impl TaskCtl {
    pub fn progress(&self, fraction: f32) {
        let _ = self.tx.send(Msg::Progress(fraction.clamp(0.0, 1.0)));
        self.ctx.request_repaint();
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

struct Running {
    rx: Receiver<Msg>,
    cancel: Arc<AtomicBool>,
    label: String,
    progress: Option<f32>,
}

/// Poganja eno opravilo naenkrat; vsa opravila si delijo isti LCG.
pub struct Runner {
    pub lcg: Arc<Mutex<Lcg>>,
    ctx: egui::Context,
    current: Option<Running>,
}

impl Runner {
    pub fn new(ctx: egui::Context, seed: u32) -> Self {
        Self { lcg: Arc::new(Mutex::new(Lcg::new(seed))), ctx, current: None }
    }

    pub fn busy(&self) -> bool {
        self.current.is_some()
    }

    pub fn label(&self) -> Option<&str> {
        self.current.as_ref().map(|r| r.label.as_str())
    }

    pub fn progress(&self) -> Option<f32> {
        self.current.as_ref().and_then(|r| r.progress)
    }

    pub fn cancel(&self) {
        if let Some(r) = &self.current {
            r.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Ponastavi generator (samo ko nič ne teče, ker opravilo drži ključavnico generatorja)
    pub fn reseed(&self, seed: u32) {
        if !self.busy() {
            *self.lcg.lock().unwrap_or_else(|e| e.into_inner()) = Lcg::new(seed);
        }
    }

    pub fn spawn<F>(&mut self, label: &str, f: F)
    where
        F: FnOnce(&mut Lcg, &TaskCtl) -> Output + Send + 'static,
    {
        if self.busy() {
            return;
        }

        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let ctl = TaskCtl { tx: tx.clone(), cancel: cancel.clone(), ctx: self.ctx.clone() };
        let lcg = self.lcg.clone();
        let ctx = self.ctx.clone();

        std::thread::spawn(move || {
            let output = {
                let mut lcg = lcg.lock().unwrap_or_else(|e| e.into_inner());
                f(&mut lcg, &ctl)
            };
            let _ = tx.send(Msg::Done(output));
            ctx.request_repaint();
        });

        self.current = Some(Running { rx, cancel, label: label.to_string(), progress: None });
    }

    /// Pokliče se na vsak frame. Vrne rezultat, če je opravilo končano.
    pub fn poll(&mut self) -> Option<Output> {
        let running = self.current.as_mut()?;

        loop {
            match running.rx.try_recv() {
                Ok(Msg::Progress(p)) => running.progress = Some(p),
                Ok(Msg::Done(output)) => {
                    self.current = None;
                    return Some(output);
                }
                Err(TryRecvError::Empty) => return None,
                Err(TryRecvError::Disconnected) => {
                    // Nit je umrla brez rezultata (panic)
                    self.current = None;
                    return Some(Output::Error("The task crashed unexpectedly.".to_string()));
                }
            }
        }
    }
}
