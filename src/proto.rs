//! The stdio line protocol — one JSON request line in, one envelope line
//! out. Malformed input NEVER kills the bin: it answers a typed error
//! envelope and keeps serving. EOF exits 0.

use katgpt_core::decision_wire::DecisionResponse;
use serde::{Deserialize, Serialize};

/// Line-protocol version. Bump on any breaking envelope change.
pub const PROTO: u32 = 1;

/// The frozen reference genome this engine serves (Bench 892 champion,
/// katgpt-rs Issue 893). Asserted at engine load.
pub const GENOME_ID: &str = "68cae9d382014662";

/// Error codes — stable vocabulary, one per failure CLASS.
pub mod codes {
    pub const BAD_JSON: &str = "bad_json";
    pub const BAD_REQUEST: &str = "bad_request";
    pub const BAD_STATE: &str = "bad_state";
    pub const QUESTION_KIND: &str = "question_kind";
    pub const OPTIONS_COUNT: &str = "options_count";
    pub const INTERNAL: &str = "internal";
}

/// The success envelope: the wire response + the engine's own in-process
/// decision time, so harnesses render it beside the round-trip (never
/// display a sub-ms decision through a same-size subprocess overhead).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub proto: u32,
    /// The genome digest that served (`Genome::id()` — blake3-16).
    pub genome: String,
    pub response: DecisionResponse,
    /// The engine's in-process decision time for THIS request (serde +
    /// stdio excluded) — the zero-network measurement law.
    pub in_engine_decision_ns: u64,
}

/// The typed error envelope. The pipe stays alive after one of these.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub proto: u32,
    pub error: ProtoError,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtoError {
    /// One of [`codes`] — stable, machine-branchable.
    pub code: String,
    /// Human context (display text, never parsed).
    pub message: String,
    /// 1-based stdin line index the request arrived on.
    pub request_index: u64,
}
