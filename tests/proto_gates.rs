//! The stdio protocol contract — typed errors, never panics; EOF exits 0;
//! the response envelope carries in-engine decision time.

use katgpt_core::decision_wire::{DecisionRequest, Question};
use katgpt_tetris::sim::{Board, Piece};
use reflexer::engine::Engine;
use reflexer::proto::{GENOME_ID, codes};
use reflexer::state_codec::GameState;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

struct Pipe {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
}

impl Pipe {
    fn spawn(extra_args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_reflexer"))
            .args(extra_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn reflexer bin");
        let reader = BufReader::new(child.stdout.take().expect("stdout"));
        Self { child, reader }
    }

    fn send(&mut self, line: &str) {
        let stdin = self.child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{line}").expect("write line");
        stdin.flush().expect("flush");
    }

    fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.reader.read_line(&mut line).expect("read line");
        assert!(!line.is_empty(), "bin closed stdout early");
        serde_json::from_str(line.trim()).expect("envelope json")
    }

    fn request(&mut self, req: &DecisionRequest) -> Value {
        self.send(&serde_json::to_string(req).unwrap());
        self.recv()
    }

    fn close_stdin(&mut self) {
        if let Some(mut stdin) = self.child.stdin.take() {
            stdin.flush().unwrap();
            drop(stdin);
        }
    }

    fn wait_exit(&mut self) -> i32 {
        self.close_stdin();
        self.child.wait().expect("wait").code().unwrap_or(-1)
    }
}

fn place_request(engine: &Engine) -> DecisionRequest {
    let state = GameState::encode(&Board::empty(), Piece::I, Piece::O, None, true, &[]);
    let sim = state.to_sim().unwrap();
    let labels = engine.option_labels(&sim);
    DecisionRequest {
        state: serde_json::to_string(&state).unwrap(),
        questions: vec![Question::choice(
            "place",
            "place the current piece",
            labels,
            None,
        )],
    }
}

fn assert_ok_place(envelope: &Value, labels: usize) -> Value {
    assert_eq!(envelope["proto"], 1, "proto version");
    assert_eq!(envelope["genome"], GENOME_ID, "genome pin");
    let ns = envelope["in_engine_decision_ns"].as_u64().expect("ns");
    assert!(ns > 0, "in-engine decision time must be recorded");
    let resp = &envelope["response"];
    let answers = resp["answers"].as_array().expect("answers");
    assert_eq!(answers.len(), 1);
    let ans = &answers[0];
    assert_eq!(ans["question_id"], "place");
    let index = ans["outcome"]["choice"]["index"]
        .as_u64()
        .expect("choice index");
    assert!((index as usize) < labels, "index {index} in bounds");
    let probs = ans["probabilities"].as_array().expect("probs");
    assert_eq!(probs.len(), labels, "full-arity probabilities");
    let sum: f64 = probs.iter().map(|p| p.as_f64().unwrap()).sum();
    assert!((sum - 1.0).abs() < 1e-3, "probs sum ~1, got {sum}");
    assert_eq!(resp["routing"]["lane"], "modelless");
    assert_eq!(resp["calibration"]["method"], "none");
    envelope.clone()
}

#[test]
fn valid_request_answers_and_validates_against_the_wire() {
    let engine = Engine::champion();
    let req = place_request(&engine);
    let mut pipe = Pipe::spawn(&[]);
    let envelope = pipe.request(&req);
    assert_ok_place(&envelope, req.questions[0].options.len());
    // Round-trip through the wire types too.
    let env: reflexer::proto::ResponseEnvelope =
        serde_json::from_value(envelope).expect("typed envelope");
    env.response
        .validate_against(&req)
        .expect("wire validation");
    assert_eq!(env.genome, GENOME_ID);
}

#[test]
fn malformed_json_gets_typed_error_then_pipe_recovers() {
    let engine = Engine::champion();
    let mut pipe = Pipe::spawn(&[]);
    pipe.send("this is not json");
    let err = pipe.recv();
    assert_eq!(err["error"]["code"], codes::BAD_JSON);
    assert!(err["error"]["request_index"].as_u64().unwrap() >= 1);

    // The pipe stays alive and answers the next valid request.
    let req = place_request(&engine);
    let envelope = pipe.request(&req);
    assert_ok_place(&envelope, req.questions[0].options.len());
    assert_eq!(pipe.wait_exit(), 0);
}

#[test]
fn wire_invalid_request_is_rejected_as_bad_request() {
    let engine = Engine::champion();
    let mut req = place_request(&engine);
    req.questions.push(req.questions[0].clone()); // duplicate id
    let mut pipe = Pipe::spawn(&[]);
    let err = pipe.request(&req);
    assert_eq!(err["error"]["code"], codes::BAD_REQUEST);
    assert_eq!(pipe.wait_exit(), 0);
}

#[test]
fn bad_state_is_typed() {
    let engine = Engine::champion();
    let mut req = place_request(&engine);
    req.state = "not json".to_string();
    let mut pipe = Pipe::spawn(&[]);
    let err = pipe.request(&req);
    assert_eq!(err["error"]["code"], codes::BAD_STATE);
    assert_eq!(pipe.wait_exit(), 0);
}

#[test]
fn options_count_mismatch_is_typed() {
    let state = GameState::encode(&Board::empty(), Piece::I, Piece::O, None, true, &[]);
    let mut req = DecisionRequest {
        state: serde_json::to_string(&state).unwrap(),
        questions: vec![Question::choice(
            "place",
            "place the current piece",
            vec!["first".to_string(), "second".to_string()],
            None,
        )],
    };
    req.state = serde_json::to_string(&state).unwrap();
    let mut pipe = Pipe::spawn(&[]);
    let err = pipe.request(&req);
    assert_eq!(err["error"]["code"], codes::OPTIONS_COUNT);
    assert_eq!(pipe.wait_exit(), 0);
}

#[test]
fn unknown_question_abstains_with_zero_confidence() {
    let mut pipe = Pipe::spawn(&[]);
    let req = DecisionRequest {
        state: "{}".to_string(),
        questions: vec![Question::noul("totally-unknown", "what is the weather?")],
    };
    let envelope = pipe.request(&req);
    let ans = &envelope["response"]["answers"][0];
    assert_eq!(ans["question_id"], "totally-unknown");
    assert!(ans["outcome"].is_null(), "abstained");
    assert_eq!(ans["confidence"].as_f64().unwrap(), 0.0);
    assert_eq!(pipe.wait_exit(), 0);
}

#[test]
fn known_id_wrong_kind_is_typed() {
    let mut pipe = Pipe::spawn(&[]);
    let req = DecisionRequest {
        state: "{}".to_string(),
        questions: vec![Question::noul("place", "mis-kinded question")],
    };
    let err = pipe.request(&req);
    assert_eq!(err["error"]["code"], codes::QUESTION_KIND);
    assert_eq!(pipe.wait_exit(), 0);
}

#[test]
fn eof_exits_zero() {
    let mut pipe = Pipe::spawn(&[]);
    assert_eq!(pipe.wait_exit(), 0);
}

#[test]
fn record_mode_writes_verifiable_manifest_and_replayable_rows() {
    let dir = std::env::temp_dir().join(format!("reflexer-proto-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let rows_path = dir.join("rows.jsonl");
    let key = dir.join("k.seed");
    unsafe { std::env::set_var("REFLEXER_SUBMISSION_KEY", &key) };

    let engine = Engine::champion();
    let req = place_request(&engine);
    let mut pipe = Pipe::spawn(&["--record", rows_path.to_str().unwrap()]);
    let envelope = pipe.request(&req);
    assert_ok_place(&envelope, req.questions[0].options.len());
    pipe.request(&req);
    assert_eq!(pipe.wait_exit(), 0);

    // Manifest verifies over the exact rows bytes.
    let manifest: reflexer::record::Manifest =
        serde_json::from_slice(&std::fs::read(rows_path.with_extension("manifest.json")).unwrap())
            .unwrap();
    let rows_bytes = std::fs::read(&rows_path).unwrap();
    assert_eq!(manifest.rows, 2);
    reflexer::record::verify_manifest(&manifest, &rows_bytes).expect("manifest verifies");

    // Rows are replayable: feeding each row's request through the
    // in-process engine reproduces the recorded outcomes.
    for line in String::from_utf8(rows_bytes).unwrap().lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let request: DecisionRequest = serde_json::from_value(row["request"].clone()).unwrap();
        let (resp, _) = engine.answer(&request).unwrap();
        for (a, r) in resp.answers.iter().zip(row["outcomes"].as_array().unwrap()) {
            assert_eq!(
                serde_json::to_value(a.outcome).unwrap(),
                r["outcome"],
                "replay must reproduce the recorded outcome"
            );
        }
    }
    std::fs::remove_dir_all(&dir).ok();
}
