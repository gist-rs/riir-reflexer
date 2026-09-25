//! One request line → one envelope line — the transport-free core of the
//! line protocol, shared by every host that is not stdio (the wasm build
//! behind the in-tab arena lane and the Cloudflare Worker).
//!
//! Same law as the bin: malformed input answers a typed error envelope,
//! never a panic; a well-formed request answers the engine's response plus
//! its in-process decision time.

use crate::engine::Engine;
use crate::proto::{ErrorEnvelope, PROTO, ProtoError, ResponseEnvelope, codes};
use katgpt_core::decision_wire::{DecisionRequest, DecisionResponse};

/// A typed line failure (code + message) — never a panic.
#[derive(Clone, Debug, PartialEq)]
pub struct LineError {
    pub code: &'static str,
    pub message: String,
}

/// Decode, validate and answer one request line.
pub fn handle_line(
    engine: &Engine,
    line: &str,
) -> Result<(DecisionRequest, DecisionResponse, u64), LineError> {
    let request: DecisionRequest = serde_json::from_str(line).map_err(|e| LineError {
        code: codes::BAD_JSON,
        message: e.to_string(),
    })?;
    request.validate().map_err(|e| LineError {
        code: codes::BAD_REQUEST,
        message: format!("{e:?}"),
    })?;
    // Defense in depth: the engine is deterministic and should never panic,
    // but where unwinding exists a panic must answer an envelope. (Under
    // panic=abort — every wasm build — a panic traps; the host re-creates
    // the instance and answers `internal` itself.)
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine.answer(&request)));
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

/// [`handle_line`] rendered as the wire envelope JSON — success or typed
/// error, exactly the bin's stdout line for the same input.
pub fn envelope_line(engine: &Engine, line: &str, request_index: u64) -> String {
    match handle_line(engine, line) {
        Ok((_, response, ns)) => serde_json::to_string(&ResponseEnvelope {
            proto: PROTO,
            genome: engine.genome_id(),
            response,
            in_engine_decision_ns: ns,
        }),
        Err(err) => serde_json::to_string(&ErrorEnvelope {
            proto: PROTO,
            error: ProtoError {
                code: err.code.to_string(),
                message: err.message,
                request_index,
            },
        }),
    }
    .expect("envelopes serialize")
}
