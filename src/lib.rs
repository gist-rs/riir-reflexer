//! reflexer — the public decision ENGINE (this repo's `.proposals/001`).
//!
//! Runs genomes over game sims and answers `decision_wire` requests:
//! `choice` over enumerated landing options, `score` for state evaluation,
//! `noul` for the yes/no class, abstention first-class. The substrate is
//! consumed, never duplicated: the tetris board sim + lookahead + rulebook
//! evaluator + the frozen Bench-892 reference genome all live in
//! `katgpt-tetris` (katgpt-rs Issue 893).
//!
//! Layout:
//! - [`state_codec`] — the wire `state` JSON schema (the measurement
//!   contract's state encoding; exact board round-trip).
//! - [`readout`] — the Bench-817 confidence readout dispatch, inherited.
//! - [`proto`] — the stdio line-JSON envelopes (response carries
//!   `in_engine_decision_ns`; errors are typed, never panics).
//! - [`engine`] — the engine: reference genome + question mapping.
//! - [`record`] — the opt-in trajectory submission client (signs rows;
//!   zero billing code, per-machine key only).
//!
//! Zero network calls anywhere. Zero riir-* dependencies (the leaf law).

pub mod engine;
pub mod lane;
pub mod proto;
pub mod readout;
pub mod record;
pub mod state_codec;
