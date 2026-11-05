use num_bigint::BigUint;

/// Sporočilo v vrstici na dnu okna
#[derive(Default)]
pub struct Status {
    text: String,
    is_error: bool,
}

impl Status {
    pub fn info(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.is_error = false;
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.is_error = true;
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }

    pub fn get(&self) -> Option<(bool, &str)> {
        if self.text.is_empty() { None } else { Some((self.is_error, self.text.as_str())) }
    }
}

/// Prebere naravno število poljubne velikosti (dovoli presledke in podčrtaje kot ločila tisočic)
pub fn parse_big(text: &str, what: &str) -> Result<BigUint, String> {
    let cleaned: String = text.chars().filter(|c| !c.is_whitespace() && *c != '_').collect();
    cleaned.parse::<BigUint>().map_err(|_| format!("{} must be a non-negative whole number.", what))
}

pub fn format_micros(micros: f64) -> String {
    if micros < 1.0 {
        format!("{:.0} ns", micros * 1000.0)
    } else if micros < 1000.0 {
        format!("{:.1} µs", micros)
    } else if micros < 1_000_000.0 {
        format!("{:.2} ms", micros / 1000.0)
    } else {
        format!("{:.2} s", micros / 1_000_000.0)
    }
}

/// Besedilo, ki ga ni mogoče urejati, a ga je mogoče označiti in kopirati (za dolge številke)
pub fn copyable(ui: &mut egui::Ui, label: &str, text: &str) {
    ui.label(label);
    let mut shown = text;
    ui.add(
        egui::TextEdit::multiline(&mut shown)
            .desired_rows(1)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace),
    );
}
