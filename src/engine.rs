//! The engine: the frozen reference genome + the question mapping.
//!
//! One genome serves the whole bin at P2 — the Bench-892 hybrid champion
//! (vessel artifact loading is P3). Every question kind maps onto the same
//! searched vector ([`katgpt_tetris::rulebook::decide_scored`]) so ONE
//! search answers a full request:
//!
//! - `place` (`choice`) — options are the EXACT candidate order of
//!   `decide_scored` (no-hold first, then the hold swap, landing order).
//!   The outcome index is the first STRICT argmax — the same fold
//!   [`decide`] is, so wire games are bit-identical to `play_game`.
//!   Probabilities are sigmoid-margin weights at the population-std scale
//!   (the tetris_09 site-walk convention — sigmoid, never softmax).
//! - `state` (`score`) — the position's value-to-go (best achievable root
//!   value) through σ(v / [`V_REF`]) onto the 5-level rubric, with a
//!   piecewise-linear distribution between adjacent level centers.
//! - `survive` (`noul`) — "does a legal placement exist": yes = the
//!   candidate set is non-empty; `p_yes` = σ(v / [`V_REF`]).
//! - unknown question id → ABSTAIN (confidence 0): abstention is
//!   first-class on this wire, and "cannot answer" is the honest answer.
//! - no candidates (top out) → `place` ABSTAINS; `state` scores critical;
//!   `survive` answers no.
//!
//! Abstention policy is STRUCTURAL (the gate is "the engine has no legal
//! answer"), never a confidence threshold — a threshold would sit inside
//! game replay and break bit-identity with the substrate battery.

use crate::proto::GENOME_ID;
use crate::readout;
use crate::state_codec::{GameState, SimState};
use katgpt_core::decision_wire::{
    Answer, Calibration, DecisionRequest, DecisionResponse, Lane, Question, QuestionKind, Routing,
};
use katgpt_tetris::rulebook::{self, Genome, View};
use std::time::Instant;

/// The rubric for `score` questions — LOWEST level first (the wire law).
pub const SCORE_RUBRIC: [&str; 5] = ["critical", "poor", "fair", "good", "excellent"];

/// The value→quality map's reference scale (operating policy): q = σ(v /
/// V_REF). Calibrated so a healthy mid-game position lands mid-rubric.
pub const V_REF: f64 = 200.0;

/// The rulebook's top-out convention (rulebook.rs `TOPOUT`, mirrored here
/// for the death branch — the constant is part of the frozen reference).
pub const DEATH_VALUE: f64 = -1.0e12;

/// Engine-side typed failures → proto error codes.
#[derive(Clone, Debug, PartialEq)]
pub enum EngineError {
    /// The `state` field did not decode.
    BadState(String),
    /// A known question id arrived with the wrong kind.
    QuestionKind { id: String, kind: &'static str },
    /// A `place` question's option count diverged from the engine's
    /// canonical enumeration — fail-closed (bit-identity demands one
    /// enumeration).
    OptionsCount { want: usize, got: usize },
}

impl EngineError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadState(_) => crate::proto::codes::BAD_STATE,
            Self::QuestionKind { .. } => crate::proto::codes::QUESTION_KIND,
            Self::OptionsCount { .. } => crate::proto::codes::OPTIONS_COUNT,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::BadState(e) => format!("state decode failed: {e}"),
            Self::QuestionKind { id, kind } => {
                format!("question {id:?} must be a {kind} question")
            }
            Self::OptionsCount { want, got } => format!(
                "options diverged: engine expects {want}, request carries {got} \
                 (place: the decide_scored candidate order — no-hold landing options \
                 first, then the hold swap; score: the canonical 5-level rubric)"
            ),
        }
    }
}

pub struct Engine {
    genome: Genome,
}

impl Engine {
    /// The reference engine — the frozen Bench-892 champion. Panics if the
    /// substrate's champion id ever drifts from the pinned digest (that IS
    /// the freeze).
    pub fn champion() -> Self {
        let genome = Genome::champion_hybrid();
        assert_eq!(
            genome.id(),
            GENOME_ID,
            "substrate champion drifted from the pinned reference digest"
        );
        Self { genome }
    }

    pub fn genome_id(&self) -> String {
        self.genome.id()
    }

    /// The reference genome (the lane's oracle needs `mode_of`).
    pub fn genome(&self) -> &Genome {
        &self.genome
    }

    /// Answer a full request (all questions, one call — the wire law).
    /// Returns the response plus the in-process decision time.
    pub fn answer(
        &self,
        request: &DecisionRequest,
    ) -> Result<(DecisionResponse, u64), EngineError> {
        let t = Instant::now();
        let mut sim: Option<SimState> = None;
        let mut answers = Vec::with_capacity(request.questions.len());
        for q in &request.questions {
            answers.push(self.answer_one(&mut sim, &request.state, q)?);
        }
        let response = DecisionResponse {
            answers,
            routing: Routing {
                lane: Lane::Modelless,
                reason: Some("reflexer rulebook champion".to_string()),
            },
            calibration: Calibration::none(),
        };
        Ok((response, t.elapsed().as_nanos() as u64))
    }

    fn answer_one(
        &self,
        sim: &mut Option<SimState>,
        state_json: &str,
        q: &Question,
    ) -> Result<Answer, EngineError> {
        match (q.id.as_str(), q.kind) {
            ("place", QuestionKind::Choice) => self.place(sim, state_json, q),
            ("state", QuestionKind::Score) => self.score(sim, state_json, q),
            ("survive", QuestionKind::Noul) => self.noul(sim, state_json, q),
            ("place", _) => Err(EngineError::QuestionKind {
                id: q.id.clone(),
                kind: "choice",
            }),
            ("state", _) => Err(EngineError::QuestionKind {
                id: q.id.clone(),
                kind: "score",
            }),
            ("survive", _) => Err(EngineError::QuestionKind {
                id: q.id.clone(),
                kind: "noul",
            }),
            // Unknown question: the honest answer is abstention, not an
            // error — the request is well-formed, the engine just has no
            // knowledge here (abstention is first-class on this wire).
            _ => Ok(Answer::abstain(q.id.clone(), 0.0)),
        }
    }

    /// Decode the state once per request, lazily (a request whose
    /// questions are all unknown ids never pays the decode).
    fn sim<'a>(
        sim: &'a mut Option<SimState>,
        state_json: &str,
    ) -> Result<&'a SimState, EngineError> {
        if sim.is_none() {
            let gs: GameState = serde_json::from_str(state_json)
                .map_err(|e| EngineError::BadState(e.to_string()))?;
            *sim = Some(
                gs.to_sim()
                    .map_err(|e| EngineError::BadState(e.to_string()))?,
            );
        }
        Ok(sim.as_ref().unwrap())
    }

    /// The searched candidate vector for the state — ONE search shared by
    /// every question kind in the request.
    fn searched(&self, sim: &SimState) -> Vec<(rulebook::Decision, f64)> {
        let view = View {
            board: &sim.board,
            cur: sim.cur,
            next: sim.next,
            held: sim.held,
            hold_ready: sim.hold_ready,
            bag_remaining: &sim.bag,
        };
        rulebook::decide_scored(&self.genome, &view)
    }

    fn place(
        &self,
        sim: &mut Option<SimState>,
        state_json: &str,
        q: &Question,
    ) -> Result<Answer, EngineError> {
        let sim = Self::sim(sim, state_json)?;
        let scored = self.searched(sim);
        if scored.is_empty() {
            // Top out: no legal answer exists — abstain (the gate).
            return Ok(Answer::abstain(q.id.clone(), 0.0));
        }
        if q.options.len() != scored.len() {
            return Err(EngineError::OptionsCount {
                want: scored.len(),
                got: q.options.len(),
            });
        }
        // First STRICT argmax — exactly `decide`'s fold.
        let (mut best_at, mut best_v) = (0usize, f64::NEG_INFINITY);
        for (at, (_, v)) in scored.iter().enumerate() {
            if *v > best_v {
                best_v = *v;
                best_at = at;
            }
        }
        let vals: Vec<f64> = scored.iter().map(|(_, v)| *v).collect();
        let probs = sigmoid_margin_probs(&vals);
        let confidence = readout::confidence(&probs);
        Ok(Answer::choice(
            q.id.clone(),
            best_at as u32,
            probs,
            confidence,
        ))
    }

    fn score(
        &self,
        sim: &mut Option<SimState>,
        state_json: &str,
        q: &Question,
    ) -> Result<Answer, EngineError> {
        let sim = Self::sim(sim, state_json)?;
        if q.options.len() != SCORE_RUBRIC.len() {
            return Err(EngineError::OptionsCount {
                want: SCORE_RUBRIC.len(),
                got: q.options.len(),
            });
        }
        let scored = self.searched(sim);
        let v = scored
            .iter()
            .map(|(_, v)| *v)
            .fold(f64::NEG_INFINITY, f64::max);
        let v = if scored.is_empty() { DEATH_VALUE } else { v };
        let q_quality = sigmoid(v / V_REF) as f32;
        let level =
            ((q_quality * SCORE_RUBRIC.len() as f32).floor() as usize).min(SCORE_RUBRIC.len() - 1);
        let probs = level_probs(q_quality);
        let confidence = readout::confidence(&probs);
        Ok(Answer::score(q.id.clone(), level as u32, probs, confidence))
    }

    fn noul(
        &self,
        sim: &mut Option<SimState>,
        state_json: &str,
        q: &Question,
    ) -> Result<Answer, EngineError> {
        let sim = Self::sim(sim, state_json)?;
        let scored = self.searched(sim);
        let (yes, p_yes) = if scored.is_empty() {
            (false, sigmoid(DEATH_VALUE / V_REF) as f32)
        } else {
            let v = scored
                .iter()
                .map(|(_, v)| *v)
                .fold(f64::NEG_INFINITY, f64::max);
            (true, sigmoid(v / V_REF) as f32)
        };
        let confidence = readout::confidence(&[p_yes, 1.0 - p_yes]);
        Ok(Answer::noul(q.id.clone(), yes, p_yes, confidence))
    }

    /// The canonical `place` option labels for a state — the EXACT
    /// candidate order of `decide_scored`, labeled `h{hold}i{index}`.
    /// A client building a `place` question MUST enumerate in this order
    /// (the outcome index and the options-count gate both assume it).
    pub fn option_labels(&self, sim: &SimState) -> Vec<String> {
        self.searched(sim)
            .iter()
            .map(|(d, _)| format!("h{}i{}", d.use_hold as u8, d.index))
            .collect()
    }
}

#[inline]
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Sigmoid-margin weights at the population-std scale — the tetris_09
/// site-walk convention, inherited (sigmoid, never softmax): the pick
/// reads 0.5·(normalized), everything else strictly below by margin.
pub fn sigmoid_margin_probs(vals: &[f64]) -> Vec<f32> {
    let n = vals.len() as f64;
    let mean = vals.iter().sum::<f64>() / n;
    let var: f64 = vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n;
    let scale = var.sqrt().max(1e-9);
    let vmax = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let weights: Vec<f64> = vals.iter().map(|v| sigmoid((v - vmax) / scale)).collect();
    let sum: f64 = weights.iter().sum();
    weights.iter().map(|w| (w / sum) as f32).collect()
}

/// Piecewise-linear distribution over the 5 rubric levels: mass ∝
/// `max(0, 1 − K·|q − center_k|)` — deterministic, continuous, no fake
/// precision.
fn level_probs(q: f32) -> Vec<f32> {
    let k = SCORE_RUBRIC.len() as f32;
    let weights: Vec<f32> = (0..SCORE_RUBRIC.len())
        .map(|i| {
            let center = (i as f32 + 0.5) / k;
            (1.0 - k * (q - center).abs()).max(0.0)
        })
        .collect();
    let sum: f32 = weights.iter().sum();
    weights.iter().map(|w| w / sum).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_codec::GameState;
    use katgpt_tetris::sim::{Board, Piece};

    fn empty_state() -> GameState {
        GameState::encode(&Board::empty(), Piece::I, Piece::O, None, true, &[])
    }

    #[test]
    fn sigmoid_margin_probs_peak_on_max_and_sum_one() {
        let probs = sigmoid_margin_probs(&[10.0, 8.0, -2.0]);
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert_eq!(
            probs
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .unwrap()
                .0,
            0
        );
        assert!(probs[2] < probs[1] && probs[1] < probs[0]);
    }

    #[test]
    fn level_probs_sum_one_and_peak_mid() {
        let probs = level_probs(0.5);
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert_eq!(
            probs
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .unwrap()
                .0,
            2
        );
    }

    #[test]
    fn score_of_empty_board_is_not_critical() {
        let e = Engine::champion();
        let sim = empty_state().to_sim().unwrap();
        let scored = e.searched(&sim);
        assert!(!scored.is_empty());
        let v = scored
            .iter()
            .map(|(_, v)| *v)
            .fold(f64::NEG_INFINITY, f64::max);
        let q = sigmoid(v / V_REF);
        let level = ((q * 5.0).floor() as usize).min(4);
        // An empty board with the champion should not read as critical
        // (level 0) — the calibration pin; if this fires, re-calibrate
        // V_REF from a measured value range before shipping.
        assert!(
            level >= 1,
            "empty board scored {level} (q={q:.3}, v={v:.1}) — recalibrate V_REF"
        );
    }
}
