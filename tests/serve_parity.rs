//! `serve::envelope_line` (the transport-free core the wasm build and the
//! Cloudflare Worker speak) must answer every line exactly as the stdio bin
//! does — success and every error class — modulo the in-engine timing,
//! which is a measurement, not a decision.

use katgpt_core::decision_wire::{DecisionRequest, Question};
use katgpt_tetris::sim::{Board, Piece};
use reflexer::engine::Engine;
use reflexer::serve::envelope_line;
use reflexer::state_codec::GameState;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn untimed(line: &str) -> Value {
    let mut v: Value = serde_json::from_str(line).expect("envelope json");
    if let Some(ns) = v.get_mut("in_engine_decision_ns") {
        *ns = Value::from(0);
    }
    v
}

fn lines(engine: &Engine) -> Vec<String> {
    let place = |cur, next, bag: &[Piece]| {
        let state = GameState::encode(&Board::empty(), cur, next, None, false, bag);
        let labels = engine.option_labels(&state.to_sim().unwrap());
        serde_json::to_string(&DecisionRequest {
            state: serde_json::to_string(&state).unwrap(),
            questions: vec![Question::choice("place", "place it", labels, None)],
        })
        .unwrap()
    };
    vec![
        place(Piece::O, Piece::T, &[]),
        place(Piece::I, Piece::S, &[Piece::Z, Piece::L]),
        "{not json".into(),
        r#"{"state":"{}","questions":[{"id":"place","kind":"choice","prompt":"p","options":["h0i0"],"criteria":null}]}"#.into(),
        r#"{"state":"x","questions":[{"id":"mystery","kind":"noul","prompt":"p","options":[],"criteria":null}]}"#.into(),
    ]
}

#[test]
fn envelope_line_matches_the_bin_line_for_line() {
    let engine = Engine::champion();
    let inputs = lines(&engine);
    let mut child = Command::new(env!("CARGO_BIN_EXE_reflexer"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn reflexer bin");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for l in &inputs {
            writeln!(stdin, "{l}").unwrap();
        }
    }
    drop(child.stdin.take());
    let out = BufReader::new(child.stdout.take().expect("stdout"));
    let bin: Vec<String> = out.lines().map(|l| l.unwrap()).collect();
    assert_eq!(child.wait().unwrap().code(), Some(0));
    assert_eq!(bin.len(), inputs.len(), "one envelope per request line");
    for (at, (input, bin_line)) in inputs.iter().zip(&bin).enumerate() {
        let served = envelope_line(&engine, input, at as u64 + 1);
        assert_eq!(untimed(&served), untimed(bin_line), "line {}: {input}", at + 1);
    }
}
