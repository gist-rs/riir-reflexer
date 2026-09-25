//! The bin-only measurement lane driver — subprocess the built bin, drive
//! whole games through the wire, compare against the in-process oracle.
//!
//! ONE copy shared by the G1 gate test and the `measure` example (the
//! arena posture: this repo's engine is measured exactly the way every
//! external engine is — subprocess, pinned seeds, zero network).
//!
//! Bit-identity construction: the harness mirrors `play_game`'s loop
//! exactly, replacing its `decide` call with a wire round trip. The state
//! crosses as [`GameState`] JSON; the option list is built from the
//! engine's OWN canonical enumeration (in-process) — the same vector the
//! bin re-derives, so the outcome index and the enumeration cannot drift.

use crate::engine::Engine;
use crate::state_codec::GameState;
use katgpt_core::decision_wire::{DecisionRequest, Question};
use katgpt_tetris::lookahead::{Bag, LINES_SCORE, apply};
use katgpt_tetris::rulebook::{self, GameStats};
use katgpt_tetris::sim::{Board, DropRule, Piece, landing_options_with};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

/// A typed failure surfaced by the lane (envelope errors carry their code).
#[derive(Clone, Debug, PartialEq)]
pub enum LaneError {
    Envelope { code: String, message: String },
    Outcome(String),
}

/// One long-lived bin subprocess over stdio.
pub struct BinLane {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
    /// (round-trip ns, in-engine ns) per answered request.
    pub timings: Vec<(u64, u64)>,
}

impl BinLane {
    pub fn spawn(bin: impl AsRef<std::ffi::OsStr>, extra_args: &[&str]) -> std::io::Result<Self> {
        let mut child = Command::new(bin.as_ref())
            .args(extra_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let reader = BufReader::new(child.stdout.take().expect("bin stdout"));
        Ok(Self {
            child,
            reader,
            timings: Vec::new(),
        })
    }

    /// One request → one envelope. Times the round-trip (harness-side) and
    /// records the envelope's in-engine ns beside it.
    pub fn request(
        &mut self,
        request: &DecisionRequest,
    ) -> Result<crate::proto::ResponseEnvelope, LaneError> {
        let line = serde_json::to_string(request).expect("serialize request");
        let t = Instant::now();
        {
            let stdin = self.child.stdin.as_mut().expect("bin stdin");
            writeln!(stdin, "{line}")
                .and_then(|_| stdin.flush())
                .expect("write to bin");
        }
        let mut raw = String::new();
        self.reader.read_line(&mut raw).expect("read from bin");
        let round_ns = t.elapsed().as_nanos() as u64;
        let v: serde_json::Value = serde_json::from_str(raw.trim())
            .map_err(|e| LaneError::Outcome(format!("bad envelope json: {e}")))?;
        if let Some(err) = v.get("error") {
            return Err(LaneError::Envelope {
                code: err["code"].as_str().unwrap_or("?").to_string(),
                message: err["message"].as_str().unwrap_or("").to_string(),
            });
        }
        let env: crate::proto::ResponseEnvelope = serde_json::from_value(v)
            .map_err(|e| LaneError::Outcome(format!("bad envelope: {e}")))?;
        self.timings.push((round_ns, env.in_engine_decision_ns));
        Ok(env)
    }

    /// Close stdin and require exit 0.
    pub fn finish(mut self) -> std::io::Result<()> {
        if let Some(mut stdin) = self.child.stdin.take() {
            stdin.flush()?;
            drop(stdin);
        }
        let status = self.child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(format!("bin exited {status}")))
        }
    }
}

/// The in-process oracle — `play_game`'s exact body plus the decision
/// sequence recorded (sanity-assertable against `play_game` itself).
pub fn local_play_with_decisions(
    engine: &Engine,
    seed: u64,
    cap: usize,
    start: Board,
    hold_ready: bool,
) -> (GameStats, Vec<(bool, usize)>) {
    let g = engine.genome();
    let mut bag = Bag::new(seed);
    let mut board = start;
    let mut next = bag.draw();
    let mut held: Option<Piece> = None;
    let mut st = GameStats::default();
    let mut decisions = Vec::new();
    while st.pieces < cap {
        let cur = next;
        next = bag.draw();
        let view = rulebook::View {
            board: &board,
            cur,
            next,
            held,
            hold_ready,
            bag_remaining: bag.remaining(),
        };
        st.mode_counts[g.mode_of(&board) as usize] += 1;
        let Some(d) = rulebook::decide(g, &view) else {
            break;
        };
        decisions.push((d.use_hold, d.index));
        let piece = if d.use_hold {
            st.holds += 1;
            match held {
                Some(h) => {
                    held = Some(cur);
                    h
                }
                None => {
                    held = Some(cur);
                    let p = next;
                    next = bag.draw();
                    p
                }
            }
        } else {
            cur
        };
        let opts = landing_options_with(&board, piece, DropRule::FromTop);
        let (nb, l) = apply(&board, &opts[d.index].cells);
        board = nb;
        st.lines += l;
        st.tetrises += u32::from(l == 4);
        st.points += LINES_SCORE[l.min(4) as usize];
        st.pieces += 1;
    }
    (st, decisions)
}

/// A whole game through the wire — mirrors the oracle loop with the decide
/// call replaced by a round trip. Abstain = top out (the loop breaks,
/// exactly like `decide` returning `None`).
pub fn wire_play_game(
    lane: &mut BinLane,
    engine: &Engine,
    seed: u64,
    cap: usize,
    start: Board,
    hold_ready: bool,
) -> Result<(GameStats, Vec<(bool, usize)>), LaneError> {
    let g = engine.genome();
    let mut bag = Bag::new(seed);
    let mut board = start;
    let mut next = bag.draw();
    let mut held: Option<Piece> = None;
    let mut st = GameStats::default();
    let mut decisions = Vec::new();
    while st.pieces < cap {
        let cur = next;
        next = bag.draw();
        let gs = GameState::encode(&board, cur, next, held, hold_ready, bag.remaining());
        let sim = gs.to_sim().expect("state encodes");
        let labels = engine.option_labels(&sim);
        let request = DecisionRequest {
            state: serde_json::to_string(&gs).expect("state json"),
            questions: vec![Question::choice(
                "place",
                "place the current piece",
                labels,
                None,
            )],
        };
        let env = lane.request(&request)?;
        let outcome = &env.response.answers[0].outcome;
        let Some(katgpt_core::decision_wire::Outcome::Choice { index }) = outcome else {
            break; // abstain = top out
        };
        let (use_hold, idx) = parse_label(request.questions[0].options[*index as usize].as_str())?;
        st.mode_counts[g.mode_of(&board) as usize] += 1;
        decisions.push((use_hold, idx));
        let piece = if use_hold {
            st.holds += 1;
            match held {
                Some(h) => {
                    held = Some(cur);
                    h
                }
                None => {
                    held = Some(cur);
                    let p = next;
                    next = bag.draw();
                    p
                }
            }
        } else {
            cur
        };
        let opts = landing_options_with(&board, piece, DropRule::FromTop);
        let (nb, l) = apply(&board, &opts[idx].cells);
        board = nb;
        st.lines += l;
        st.tetrises += u32::from(l == 4);
        st.points += LINES_SCORE[l.min(4) as usize];
        st.pieces += 1;
    }
    Ok((st, decisions))
}

/// `h{hold}i{index}` → the decision pair (the label is the wire's record
/// of WHICH candidate the index selected; parsing it back keeps the harness
/// honest against the engine's own enumeration).
fn parse_label(label: &str) -> Result<(bool, usize), LaneError> {
    let rest = label
        .strip_prefix('h')
        .ok_or_else(|| LaneError::Outcome(format!("bad label {label:?}")))?;
    let (h, i) = rest
        .split_once('i')
        .ok_or_else(|| LaneError::Outcome(format!("bad label {label:?}")))?;
    let use_hold = match h {
        "0" => false,
        "1" => true,
        _ => return Err(LaneError::Outcome(format!("bad label {label:?}"))),
    };
    let index: usize = i
        .parse()
        .map_err(|_| LaneError::Outcome(format!("bad label {label:?}")))?;
    Ok((use_hold, index))
}

pub fn stats_eq(a: &GameStats, b: &GameStats) -> bool {
    a.pieces == b.pieces
        && a.lines == b.lines
        && a.points == b.points
        && a.tetrises == b.tetrises
        && a.holds == b.holds
        && a.mode_counts == b.mode_counts
}
