//! Meritve časovne zahtevnosti in analiza generatorja (brez uporabniškega vmesnika).

use std::time::Instant;
use num_bigint::BigUint;

use crate::gui::task::TaskCtl;
use crate::lcg::Lcg;
use crate::math;
use crate::rsa::{self, PrimeMethod, NAIVE_MAX_BITS};

pub struct Series {
    pub name: String,
    pub points: Vec<[f64; 2]>,
}

pub struct ChartData {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub series: Vec<Series>,
    pub note: String,
}

fn micros(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1e6
}

/// Naivna metoda proti Miller-Rabinu za n-bitna praštevila, n = min_bits..=max_bits (povprečje čez `reps` ponovitev)
pub fn prime_generation_time(lcg: &mut Lcg, min_bits: usize, max_bits: usize, reps: usize, s: usize, ctl: &TaskCtl) -> ChartData {
    let mut naive = Vec::new();
    let mut miller_rabin = Vec::new();
    let total = (max_bits - min_bits + 1) as f32;

    for (i, bits) in (min_bits..=max_bits).enumerate() {
        if ctl.cancelled() { break; }

        for (method, points) in [(PrimeMethod::Naive, &mut naive), (PrimeMethod::MillerRabin, &mut miller_rabin)] {
            let start = Instant::now();
            for _ in 0..reps {
                std::hint::black_box(rsa::generate_prime(lcg, bits, method, s));
            }
            points.push([bits as f64, micros(start) / reps as f64]);
        }
        ctl.progress((i + 1) as f32 / total);
    }

    ChartData {
        title: "Prime generation time: naive vs Miller-Rabin".to_string(),
        x_label: "bits of the prime".to_string(),
        y_label: "time".to_string(),
        series: vec![
            Series { name: "Naive".to_string(), points: naive },
            Series { name: format!("Miller-Rabin (s = {})", s), points: miller_rabin },
        ],
        note: format!("Average of {} generations per point, each with a different random number.", reps),
    }
}

/// Čas tvorbe 32-bitnega praštevila z Miller-Rabinom glede na parameter s
pub fn miller_rabin_time_vs_s(lcg: &mut Lcg, bits: usize, min_s: usize, max_s: usize, reps: usize, ctl: &TaskCtl) -> ChartData {
    let mut points = Vec::new();
    let total = (max_s - min_s + 1) as f32;

    for (i, s) in (min_s..=max_s).enumerate() {
        if ctl.cancelled() { break; }

        let start = Instant::now();
        for _ in 0..reps {
            std::hint::black_box(rsa::generate_prime(lcg, bits, PrimeMethod::MillerRabin, s));
        }
        points.push([s as f64, micros(start) / reps as f64]);
        ctl.progress((i + 1) as f32 / total);
    }

    ChartData {
        title: format!("Miller-Rabin: time to generate a {}-bit prime vs parameter s", bits),
        x_label: "parameter s".to_string(),
        y_label: "time".to_string(),
        series: vec![Series { name: format!("{}-bit prime", bits), points }],
        note: format!("Average of {} generations per point.", reps),
    }
}

/// Čas tvorbe ključev glede na število bitov n; vrne en graf za naivno metodo in enega za Miller-Rabina
pub fn keygen_time(lcg: &mut Lcg, min_bits: usize, max_bits: usize, reps: usize, s: usize, ctl: &TaskCtl) -> Vec<ChartData> {
    let mut charts = Vec::new();
    let total = (max_bits - min_bits + 1) as f32 * 2.0;
    let mut done = 0.0;

    for method in [PrimeMethod::Naive, PrimeMethod::MillerRabin] {
        let mut points = Vec::new();

        for bits in min_bits..=max_bits {
            if ctl.cancelled() { break; }
            done += 1.0;
            if method == PrimeMethod::Naive && bits > NAIVE_MAX_BITS { continue; }

            let mut n_bits_sum = 0.0;
            let mut time_sum = 0.0;
            for _ in 0..reps {
                let start = Instant::now();
                let keys = rsa::key_generator(lcg, bits, method, s);
                time_sum += micros(start);
                if let Ok(keys) = keys { n_bits_sum += keys.n.bits() as f64; }
            }
            points.push([n_bits_sum / reps as f64, time_sum / reps as f64]);
            ctl.progress(done / total);
        }

        charts.push(ChartData {
            title: format!("Key generation time ({})", method.name()),
            x_label: "bits of n = p·q".to_string(),
            y_label: "time".to_string(),
            series: vec![Series { name: method.name().to_string(), points }],
            note: format!("p and q have 3..{} bits. Average of {} key pairs per point (a new pair of primes each time).", max_bits, reps),
        });
    }
    charts
}

/// Čas kodiranja in dekodiranja (modulskega potenciranja) poljubnega števila glede na število bitov n
pub fn crypt_time(lcg: &mut Lcg, min_bits: usize, max_bits: usize, reps: usize, s: usize, ctl: &TaskCtl) -> ChartData {
    let one = BigUint::from(1u32);
    let mut encrypt = Vec::new();
    let mut decrypt = Vec::new();
    let mut failures = 0;
    let total = (max_bits - min_bits + 1) as f32;

    for (i, bits) in (min_bits..=max_bits).enumerate() {
        if ctl.cancelled() { break; }

        let Ok(keys) = rsa::key_generator(lcg, bits, PrimeMethod::MillerRabin, s) else { continue };
        let upper = &keys.n - &one;

        let (mut enc_time, mut dec_time) = (0.0, 0.0);
        for _ in 0..reps {
            let m = lcg.random(&one, &upper); // vsakič drugačno sporočilo M v [1, n-1]

            let start = Instant::now();
            let c = math::modular_exponetiation(&m, &keys.e, &keys.n);
            enc_time += micros(start);

            let start = Instant::now();
            let m2 = math::modular_exponetiation(&c, &keys.d, &keys.n);
            dec_time += micros(start);

            if m2 != m { failures += 1; }
        }

        let x = keys.n.bits() as f64;
        encrypt.push([x, enc_time / reps as f64]);
        decrypt.push([x, dec_time / reps as f64]);
        ctl.progress((i + 1) as f32 / total);
    }

    let mut note = format!("One key per size, {} different random messages M per point.", reps);
    if failures > 0 {
        note += &format!(" WARNING: {} messages did not decrypt correctly!", failures);
    } else {
        note += " Every message decrypted back correctly.";
    }

    ChartData {
        title: "Encryption / decryption time of one number".to_string(),
        x_label: "bits of n".to_string(),
        y_label: "time".to_string(),
        series: vec![
            Series { name: "Encrypt  M^e mod n".to_string(), points: encrypt },
            Series { name: "Decrypt  C^d mod n".to_string(), points: decrypt },
        ],
        note,
    }
}

pub struct Histogram {
    pub lo: u32,
    pub hi: u32,
    pub samples: u64,
    pub counts: Vec<u64>,
    pub expected: Vec<f64>,
    /// Število vrednosti, ki spadajo v posamezen predal (1 če je predal ena vrednost)
    pub values_per_bin: Vec<u64>,
    pub chi_squared: f64,
}

/// Naredi `samples` števil z RANDOM(lo, hi) in jih prešteje v predale (največ 1000 predalov)
pub fn histogram(lcg: &mut Lcg, lo: u32, hi: u32, samples: u64, ctl: &TaskCtl) -> Histogram {
    let range = hi as u64 - lo as u64 + 1;
    let bins = range.min(1000);
    let mut counts = vec![0u64; bins as usize];

    for i in 0..samples {
        let value = lcg.random_u32(lo, hi);
        let bin = (value as u64 - lo as u64) * bins / range;
        counts[bin as usize] += 1;

        if i % 100_000 == 0 { ctl.progress(i as f32 / samples as f32); }
    }

    // Koliko vrednosti pade v posamezen predal (ko range ni deljiv z bins, predali niso enako veliki)
    let edge = |b: u64| (b * range).div_ceil(bins);
    let values_per_bin: Vec<u64> = (0..bins).map(|b| edge(b + 1) - edge(b)).collect();
    let expected: Vec<f64> = values_per_bin.iter().map(|&v| samples as f64 * v as f64 / range as f64).collect();

    let chi_squared = counts.iter().zip(&expected)
        .map(|(&observed, &exp)| (observed as f64 - exp).powi(2) / exp)
        .sum();

    Histogram { lo, hi, samples, counts, expected, values_per_bin, chi_squared }
}
