//! Pitch detection and equal-tempered tuning, independent of the browser/UI.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pitch {
    pub frequency: f64,
    pub level: f64,
}

pub fn frequency(midi: u8, reference: f64) -> f64 {
    reference * 2_f64.powf((midi as f64 - 69.0) / 12.0)
}

pub fn cents(measured: f64, target: f64) -> f64 {
    1200.0 * (measured / target).log2()
}

pub fn nearest_string(measured: f64, strings: &[u8], reference: f64) -> usize {
    strings
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            cents(measured, frequency(**a, reference))
                .abs()
                .total_cmp(&cents(measured, frequency(**b, reference)).abs())
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

/// YIN's cumulative mean normalized difference rejects harmonics and noise.
pub fn detect(samples: &[f32], sample_rate: f64) -> Option<Pitch> {
    if samples.len() < 512
        || !sample_rate.is_finite()
        || sample_rate <= 0.0
        || samples.iter().any(|v| !v.is_finite())
    {
        return None;
    }
    let mean = samples.iter().map(|v| *v as f64).sum::<f64>() / samples.len() as f64;
    let level = (samples
        .iter()
        .map(|v| (*v as f64 - mean).powi(2))
        .sum::<f64>()
        / samples.len() as f64)
        .sqrt();
    if level < 0.003 {
        return None;
    }
    let max_lag = ((sample_rate / 55.0) as usize).min(samples.len() / 2 - 1);
    let min_lag = (sample_rate / 1100.0) as usize;
    if min_lag < 2 || min_lag >= max_lag {
        return None;
    }
    let window = samples.len() - max_lag;
    let mut difference = vec![0.0; max_lag + 1];
    for lag in 1..=max_lag {
        difference[lag] = (0..window)
            .map(|i| (samples[i] as f64 - samples[i + lag] as f64).powi(2))
            .sum();
    }
    let mut sum = 0.0;
    let mut normalized = vec![1.0; max_lag + 1];
    for lag in 1..=max_lag {
        sum += difference[lag];
        if sum > 0.0 {
            normalized[lag] = difference[lag] * lag as f64 / sum;
        }
    }
    let mut lag = min_lag;
    while lag < max_lag {
        if normalized[lag] < 0.15 {
            while lag + 1 < max_lag && normalized[lag + 1] < normalized[lag] {
                lag += 1;
            }
            let (left, center, right) = (normalized[lag - 1], normalized[lag], normalized[lag + 1]);
            let denominator = left - 2.0 * center + right;
            let offset = if denominator.abs() > 1e-12 {
                0.5 * (left - right) / denominator
            } else {
                0.0
            };
            let frequency = sample_rate / (lag as f64 + offset.clamp(-1.0, 1.0));
            return (55.0..=1100.0)
                .contains(&frequency)
                .then_some(Pitch { frequency, level });
        }
        lag += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wave(hz: f64, harmonics: bool) -> Vec<f32> {
        (0..2048)
            .map(|i| {
                let phase = std::f64::consts::TAU * hz * i as f64 / 12000.0;
                (0.2 * phase.sin()
                    + if harmonics {
                        0.35 * (2.0 * phase).sin() + 0.15 * (3.0 * phase).sin()
                    } else {
                        0.0
                    }) as f32
            })
            .collect()
    }
    #[test]
    fn detects_every_guitar_and_ukulele_string() {
        for midi in [
            36, 38, 39, 40, 43, 44, 45, 48, 49, 50, 53, 54, 55, 57, 58, 59, 60, 62, 63, 64, 67, 69,
        ] {
            let expected = frequency(midi, 440.0);
            for harmonics in [false, true] {
                let measured = detect(&wave(expected, harmonics), 12000.0)
                    .unwrap()
                    .frequency;
                assert!(
                    cents(measured, expected).abs() < 3.0,
                    "{expected}: {measured}"
                );
            }
        }
    }
    #[test]
    fn rejects_silence_invalid_and_noise() {
        assert!(detect(&vec![0.0; 2048], 12000.0).is_none());
        assert!(detect(&vec![0.5; 2048], 12000.0).is_none());
        assert!(detect(&vec![f32::NAN; 2048], 12000.0).is_none());
        assert!(detect(&wave(440.0, false), 0.0).is_none());
        let mut seed = 42u32;
        let noise: Vec<f32> = (0..2048)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                seed as f32 / u32::MAX as f32 - 0.5
            })
            .collect();
        assert!(detect(&noise, 12000.0).is_none());
    }
    #[test]
    fn measures_cents_and_selects_reentrant_strings() {
        assert!((cents(440.0 * 2_f64.powf(10.0 / 1200.0), 440.0) - 10.0).abs() < 1e-9);
        assert_eq!(nearest_string(261.63, &[67, 60, 64, 69], 440.0), 1);
        assert_eq!(nearest_string(392.0, &[67, 60, 64, 69], 440.0), 0);
        assert_eq!(frequency(69, 442.0), 442.0);
    }
}
