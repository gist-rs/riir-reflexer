//! G1 — champion replay bit-identity through the wire.
//!
//! The bin's per-seed decision sequences and game stats must equal the
//! in-process oracle (`play_game`'s exact loop) on THIS arch — the wire,
//! the state codec, and the subprocess add zero semantic drift. Held at
//! BOTH geometries: the `play_game` posture (hold on) and the h2h
//! no-hold posture (`pick_no_hold`'s view shape).

use katgpt_tetris::rulebook::play_game;
use katgpt_tetris::sim::Board;
use reflexer::engine::Engine;
use reflexer::lane::{BinLane, local_play_with_decisions, stats_eq, wire_play_game};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflexer")
}

#[test]
fn champion_digest_is_pinned() {
    assert_eq!(Engine::champion().genome_id(), reflexer::proto::GENOME_ID);
}

#[test]
fn oracle_matches_play_game_exactly() {
    // Sanity: the local recording loop IS play_game (decisions recorded,
    // nothing else). If this fails, the oracle itself drifted.
    let engine = Engine::champion();
    for seed in 1..=4u64 {
        let (stats, _) = local_play_with_decisions(&engine, seed, 240, Board::empty(), true);
        let reference = play_game(engine.genome(), seed, 240, Board::empty());
        assert!(
            stats_eq(&stats, &reference),
            "oracle drifted from play_game at seed {seed}"
        );
    }
}

#[test]
fn wire_games_match_the_oracle_hold_posture() {
    let engine = Engine::champion();
    let mut lane = BinLane::spawn(bin(), &[]).expect("spawn bin");
    for seed in 1..=6u64 {
        let (wire_stats, wire_decisions) =
            wire_play_game(&mut lane, &engine, seed, 240, Board::empty(), true).expect("wire game");
        let (stats, decisions) =
            local_play_with_decisions(&engine, seed, 240, Board::empty(), true);
        assert_eq!(
            wire_decisions, decisions,
            "seed {seed}: decision sequence diverged"
        );
        assert!(stats_eq(&wire_stats, &stats), "seed {seed}: stats diverged");
    }
    lane.finish().expect("clean exit");
}

#[test]
fn wire_games_match_the_oracle_no_hold_posture() {
    let engine = Engine::champion();
    let mut lane = BinLane::spawn(bin(), &[]).expect("spawn bin");
    for seed in 1..=4u64 {
        let (wire_stats, wire_decisions) =
            wire_play_game(&mut lane, &engine, seed, 200, Board::empty(), false)
                .expect("wire game");
        let (stats, decisions) =
            local_play_with_decisions(&engine, seed, 200, Board::empty(), false);
        assert_eq!(
            wire_decisions, decisions,
            "seed {seed}: no-hold decision sequence diverged"
        );
        assert!(
            stats_eq(&wire_stats, &stats),
            "seed {seed}: no-hold stats diverged"
        );
    }
    lane.finish().expect("clean exit");
}

#[test]
fn wire_games_match_the_oracle_on_garbage_boards() {
    // A harder start: 8 rows of 60% garbage per seed — the board states
    // the empty-board games never reach.
    let engine = Engine::champion();
    let mut lane = BinLane::spawn(bin(), &[]).expect("spawn bin");
    for seed in 11..=13u64 {
        let start = katgpt_tetris::lookahead::garbage_board(seed, 8, 60);
        let (wire_stats, wire_decisions) =
            wire_play_game(&mut lane, &engine, seed, 200, start.clone(), true).expect("wire game");
        let (stats, decisions) = local_play_with_decisions(&engine, seed, 200, start, true);
        assert_eq!(
            wire_decisions, decisions,
            "seed {seed}: garbage decision sequence diverged"
        );
        assert!(
            stats_eq(&wire_stats, &stats),
            "seed {seed}: garbage stats diverged"
        );
    }
    lane.finish().expect("clean exit");
}
