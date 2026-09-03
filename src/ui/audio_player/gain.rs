//! Live preview gain: output level is volume × 10^(dB/20), never a file rewrite.

/// Hard cap on linear amplitude sent to the backend (matches kira preview boosts).
pub const OUTPUT_LEVEL_CAP: f32 = 32.0;

/// Linear output level from user volume (`0.0..=1.0`) and gain in decibels.
#[must_use]
pub fn effective_linear_level(volume: f32, gain_db: f32) -> f32 {
    let volume = volume.clamp(0.0, 1.0);
    let linear_gain = 10f32.powf(gain_db / 20.0);
    (volume * linear_gain).clamp(0.0, OUTPUT_LEVEL_CAP)
}

#[cfg(test)]
mod tests {
    use super::{OUTPUT_LEVEL_CAP, effective_linear_level};

    #[test]
    fn transport_effective_linear_level_matches_formula() {
        let volume = 0.8_f32;
        let gain_db = 6.0_f32;
        let expected = (volume * 10f32.powf(gain_db / 20.0)).clamp(0.0, OUTPUT_LEVEL_CAP);
        let got = effective_linear_level(volume, gain_db);
        assert!(
            (got - expected).abs() < 1e-6,
            "effective linear level must equal volume * 10^(gain_db/20); got {got}, expected {expected}"
        );

        let unity = effective_linear_level(0.5, 0.0);
        assert!(
            (unity - 0.5).abs() < 1e-6,
            "0 dB must leave volume unchanged; got {unity}"
        );
    }

    #[test]
    fn transport_effective_linear_level_clamps_cap() {
        let got = effective_linear_level(1.0, 40.0);
        assert!(
            (got - OUTPUT_LEVEL_CAP).abs() < 1e-6,
            "boosts above the output cap must clamp to {OUTPUT_LEVEL_CAP}; got {got}"
        );
    }
}
