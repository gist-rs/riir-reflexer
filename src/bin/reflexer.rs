//! The reflexer bin — line-JSON over stdio.
//!
//! One request line in → one envelope line out. Malformed input answers a
//! typed error envelope and the pipe stays alive; EOF exits 0 (writing the
//! signed trajectory manifest first when `--record` is active).
//!
//! Usage:
//! ```text
//! reflexer [--record <path.jsonl>] [--version]
//! ```

use reflexer::engine::Engine;
use reflexer::proto::{ErrorEnvelope, GENOME_ID, PROTO, ProtoError, ResponseEnvelope, codes};
use std::io::{BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!(
            "reflexer {} proto={PROTO} genome={GENOME_ID}",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }
    let record_path = flag_value(&args, "--record");

    let engine = Engine::champion();
    let mut recorder = record_path.map(|p| {
        reflexer::record::TrajectoryRecorder::open(std::path::Path::new(&p))
            .expect("open trajectory record file")
    });

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for (at, line) in stdin.lock().lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("reflexer: stdin read error: {e}");
                std::process::exit(1);
            }
        };
        let request_index = at as u64 + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match handle_line(&engine, trimmed) {
            Ok((request, response, ns)) => {
                if let Some(rec) = recorder.as_mut()
                    && let Err(e) = rec.record(request, &response.answers)
                {
                    eprintln!("reflexer: trajectory record failed: {e}");
                    std::process::exit(1);
                }
                let envelope = ResponseEnvelope {
                    proto: PROTO,
                    genome: engine.genome_id(),
                    response,
                    in_engine_decision_ns: ns,
                };
                write_line(
                    &mut out,
                    &serde_json::to_string(&envelope).expect("serialize envelope"),
                );
            }
            Err(err) => {
                let envelope = ErrorEnvelope {
                    proto: PROTO,
                    error: ProtoError {
                        code: err.code.to_string(),
                        message: err.message,
                        request_index,
                    },
                };
                write_line(
                    &mut out,
                    &serde_json::to_string(&envelope).expect("serialize error"),
                );
            }
        }
    }

    if let Some(rec) = recorder {
        match rec.finish(&engine.genome_id()) {
            Ok(path) => eprintln!("reflexer: trajectory manifest written: {}", path.display()),
            Err(e) => {
                eprintln!("reflexer: trajectory manifest failed: {e}");
                std::process::exit(1);
            }
        }
    }
}

/// A typed line failure (code + message) — never a panic.
struct LineError {
    code: &'static str,
    message: String,
}

fn handle_line(
    engine: &Engine,
    line: &str,
) -> Result<
    (
        katgpt_core::decision_wire::DecisionRequest,
        katgpt_core::decision_wire::DecisionResponse,
        u64,
    ),
    LineError,
> {
    let request: katgpt_core::decision_wire::DecisionRequest =
        serde_json::from_str(line).map_err(|e| LineError {
            code: codes::BAD_JSON,
            message: e.to_string(),
        })?;
    request.validate().map_err(|e| LineError {
        code: codes::BAD_REQUEST,
        message: format!("{e:?}"),
    })?;
    // Defense in depth: the engine is deterministic and should never
    // panic, but a panic must answer an envelope, not kill the pipe.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine.answer(&request)));
    match result {
        Ok(Ok((response, ns))) => Ok((request, response, ns)),
        Ok(Err(e)) => Err(LineError {
            code: e.code(),
            message: e.message(),
        }),
        Err(p) => Err(LineError {
            code: codes::INTERNAL,
            message: format!("engine panicked: {p:?}"),
        }),
    }
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn write_line(out: &mut impl Write, json: &str) {
    let _ = writeln!(out, "{json}");
    let _ = out.flush();
}
