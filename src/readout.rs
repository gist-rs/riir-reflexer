//! Confidence readout dispatch — INHERITED, never re-derived.
//!
//! Bench 817's verdict (katgpt-rs; the same inheritance riir-reflex's
//! `readout` carries): on a deterministic forward, agreement-across-rereads
//! is NOT a better ranking signal — the analytic functionals rank
//! identically on single-modal subsets and diverge on multi-modal ones.
//! The dispatch TABLE is the contract:
//!
//! - **narrow** (≤ [`NARROW_MAX_OPTIONS`] options): inverted normalized
//!   label entropy `1 − H/ln K`;
//! - **wide** (> [`NARROW_MAX_OPTIONS`]): argmax-label-prob.
//!
//! The constant is repo-owned operating policy; changing it is a policy
//! edit, never a re-derivation of Bench 817.

/// Narrow/wide dispatch bound (Bench 817's policy constant, inherited).
pub const NARROW_MAX_OPTIONS: usize = 8;

/// Normalized Shannon entropy in nats, `H / ln K` ∈ [0, 1]. `K ≤ 1` reads
/// as fully peaked (0.0).
#[inline]
fn normalized_entropy(probs: &[f32]) -> f32 {
    let k = probs.len();
    if k <= 1 {
        return 0.0;
    }
    let ln_k = (k as f32).ln();
    let h: f32 = probs
        .iter()
        .filter(|p| **p > 0.0)
        .map(|p| -p * p.ln())
        .sum();
    (h / ln_k).clamp(0.0, 1.0)
}

/// The confidence readout for one answer distribution. Always in [0, 1].
pub fn confidence(probs: &[f32]) -> f32 {
    if probs.len() <= NARROW_MAX_OPTIONS {
        1.0 - normalized_entropy(probs)
    } else {
        probs.iter().copied().fold(0.0f32, f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_uses_inverted_entropy() {
        // Uniform over 4 → fully spread → 0 confidence.
        assert!((confidence(&[0.25; 4]) - 0.0).abs() < 1e-6);
        // One-hot over 4 → fully peaked → 1.
        assert!((confidence(&[1.0, 0.0, 0.0, 0.0]) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn wide_uses_argmax_prob() {
        let many: Vec<f32> = std::iter::repeat_n(0.01f32, 20).collect();
        assert!((confidence(&many) - 0.01).abs() < 1e-6);
        let mut peaked = many;
        peaked[7] = 0.81;
        assert!((confidence(&peaked) - 0.81).abs() < 1e-6);
    }

    #[test]
    fn dispatch_boundary_is_at_eight() {
        let eight: Vec<f32> = std::iter::repeat_n(0.125f32, 8).collect();
        assert!((confidence(&eight) - 0.0).abs() < 1e-6); // narrow arm
        let nine: Vec<f32> = std::iter::repeat_n(1.0f32 / 9.0, 9).collect();
        assert!((confidence(&nine) - 1.0 / 9.0).abs() < 1e-3); // wide arm
    }
}
