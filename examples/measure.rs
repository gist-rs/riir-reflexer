//! The bin-only measurement lane — the arena posture (T5) + the G2 budget
//! gate (T8). Run under `--release` with the bin built first:
//!
//! ```text
//! cargo build --release --bin reflexer --example measure
//! cargo run --release --example measure [--seeds 10] [--cap 500] [--out DIR]
//! ```
//!
//! Zero network. Subprocess the built bin; pinned seeds; per-row artifact
//! pins (binary BLAKE3 + genome digest); G1 bit-identity proven inline
//! against the in-process oracle on every measured game; in-engine vs
//! round-trip rendered side by side. REFUSES on battery (the
//! bench-preflight law); a box with no battery (desktop) proceeds with a
//! loud "no battery" note.

use katgpt_tetris::sim::Board;
use reflexer::engine::Engine;
use reflexer::lane::{BinLane, local_play_with_decisions, stats_eq, wire_play_game};
use std::path::PathBuf;

/// G2 operating budgets (policy, with the provenance discipline). The
/// substrate's published record is 0.32–0.33 ms/decision at the h2h
/// (no-hold) geometry — the no-hold budget is that record with headroom.
/// The hold posture structurally searches ~2× the root set (the hold
/// candidate adds a second piece's landing tree), so its budget carries
/// the same headroom over ~2× the record.
const BUDGET_NOHOLD_P50_NS: u64 = 500_000;
const BUDGET_HOLD_P50_NS: u64 = 1_000_000;
const BUDGET_P99_NS: u64 = 3_000_000;

fn arg<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seeds_hold: u64 = arg(&args, "--hold-seeds")
        .and_then(|s| s.parse().ok())
        .unwrap_or(6);
    let seeds_nohold: u64 = arg(&args, "--seeds")
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let cap: usize = arg(&args, "--cap")
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);
    let out_dir = PathBuf::from(arg(&args, "--out").unwrap_or(".benchmarks/data/001"));

    let bin = resolve_bin();
    let bin_blake3 = blake3::hash(&std::fs::read(&bin).expect("read bin bytes")).to_string();
    let engine = Engine::champion();
    println!("== reflexer measure lane ==");
    println!("bin: {} (blake3 {bin_blake3})", bin.display());
    println!("genome: {}", engine.genome_id());

    let power = power_source();
    if power == Power::Battery {
        eprintln!("REFUSED: running on battery — the box state law (plug in and re-run).");
        std::process::exit(1);
    }
    let load = loadavg();
    let max_load: f64 = std::env::var("MAX_LOAD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(6.0);
    if load > max_load {
        eprintln!(
            "REFUSED: load average {load:.2} exceeds MAX_LOAD={max_load} — a sibling session is on the box \
             (raise with MAX_LOAD=n if this is deliberate; the number would measure the scheduler, not the engine)."
        );
        std::process::exit(1);
    }
    let note = match power {
        Power::Ac => "AC",
        Power::NoBattery => "AC (no battery present)",
        Power::Battery => unreachable!(),
    };
    println!(
        "PROVENANCE: {} power={note} loadavg={load} cores={}",
        std::env::consts::ARCH,
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0)
    );

    let mut lane = BinLane::spawn(&bin, &[]).expect("spawn bin");
    let mut all_ok = true;
    let mut games: Vec<serde_json::Value> = Vec::new();
    let mut posture_stats: Vec<serde_json::Value> = Vec::new();

    // The battery runs `runs` times (default 3, the Issue-723 best-of
    // discipline): shared-box contention inflates WHOLE runs (measured:
    // no-hold p50 330 → 631 µs across two runs at the same 1-min load),
    // so the budget reads the BEST per-posture p50/p99 across runs — a
    // real regression raises the floor, contention does not.
    let runs: usize = arg(&args, "--runs")
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let mut hold_best: Option<Stats> = None;
    let mut nohold_best: Option<Stats> = None;
    for run in 0..runs {
        let at0 = lane.timings.len();
        let (g1a, mut games_a) = run_posture(
            &mut lane,
            &engine,
            "hold",
            1,
            seeds_hold,
            240.min(cap),
            true,
        );
        let hold_stats = Stats::of(&lane.timings[at0..].iter().map(|t| t.1).collect::<Vec<_>>());
        let at1 = lane.timings.len();
        let (g1b, mut games_b) =
            run_posture(&mut lane, &engine, "no-hold", 1, seeds_nohold, cap, false);
        let nohold_stats = Stats::of(&lane.timings[at1..].iter().map(|t| t.1).collect::<Vec<_>>());
        println!(
            "run {}/{}: hold p50 {} µs p99 {} µs · no-hold p50 {} µs p99 {} µs",
            run + 1,
            runs,
            hold_stats.p50 / 1000,
            hold_stats.p99 / 1000,
            nohold_stats.p50 / 1000,
            nohold_stats.p99 / 1000
        );
        posture_stats.push(serde_json::json!({
            "run": run + 1,
            "hold_in_engine_ns": {"p50": hold_stats.p50, "p90": hold_stats.p90, "p99": hold_stats.p99, "max": hold_stats.max},
            "nohold_in_engine_ns": {"p50": nohold_stats.p50, "p90": nohold_stats.p90, "p99": nohold_stats.p99, "max": nohold_stats.max},
        }));
        all_ok = all_ok && g1a && g1b;
        games.append(&mut games_a);
        games.append(&mut games_b);
        hold_best = Some(match hold_best {
            Some(b) if b.p50 <= hold_stats.p50 => b,
            _ => hold_stats,
        });
        nohold_best = Some(match nohold_best {
            Some(b) if b.p50 <= nohold_stats.p50 => b,
            _ => nohold_stats,
        });
    }
    let hold_stats = hold_best.expect("at least one run");
    let nohold_stats = nohold_best.expect("at least one run");
    let games_json = serde_json::Value::Array(games);
    // Pooled round-trip stats over every measured request (taken before
    // the lane is consumed).
    let round: Vec<u64> = lane.timings.iter().map(|t| t.0).collect();
    lane.finish().expect("bin clean exit");
    let round_stats = Stats::of(&round);
    println!(
        "best-of-{runs} in-engine p50: hold {} µs · no-hold {} µs",
        hold_stats.p50 / 1000,
        nohold_stats.p50 / 1000
    );
    println!("pooled round-trip ns: {round_stats}");

    let budget_ok = hold_stats.p50 <= BUDGET_HOLD_P50_NS
        && nohold_stats.p50 <= BUDGET_NOHOLD_P50_NS
        && hold_stats.p99 <= BUDGET_P99_NS
        && nohold_stats.p99 <= BUDGET_P99_NS;
    println!(
        "G2 budget    : {} (in-engine p50: hold {} µs ≤ {}, no-hold {} µs ≤ {}; p99 ≤ {})",
        if budget_ok { "PASS" } else { "FAIL" },
        hold_stats.p50 / 1000,
        BUDGET_HOLD_P50_NS / 1000,
        nohold_stats.p50 / 1000,
        BUDGET_NOHOLD_P50_NS / 1000,
        BUDGET_P99_NS / 1000,
    );
    println!(
        "G1 bit-identity: {}",
        if all_ok {
            "PASS (every measured game matches the oracle)"
        } else {
            "FAIL"
        }
    );

    let artifact = serde_json::json!({
        "schema": "reflexer-measure-v1",
        "bin_blake3": bin_blake3,
        "genome": engine.genome_id(),
        "proto": reflexer::proto::PROTO,
        "provenance": {
            "arch": std::env::consts::ARCH,
            "power": note,
            "loadavg": load,
        },
        "budget": {
            "hold_p50_ns": hold_stats.p50,
            "nohold_p50_ns": nohold_stats.p50,
            "budget_hold_p50_ns": BUDGET_HOLD_P50_NS,
            "budget_nohold_p50_ns": BUDGET_NOHOLD_P50_NS,
            "budget_p99_ns": BUDGET_P99_NS,
            "pass": budget_ok,
        },
        "best_of_runs": runs,
        "hold_best_in_engine_ns": { "p50": hold_stats.p50, "p90": hold_stats.p90, "p99": hold_stats.p99, "max": hold_stats.max },
        "nohold_best_in_engine_ns": { "p50": nohold_stats.p50, "p90": nohold_stats.p90, "p99": nohold_stats.p99, "max": nohold_stats.max },
        "runs": posture_stats,
        "round_trip_ns": { "p50": round_stats.p50, "p90": round_stats.p90, "p99": round_stats.p99, "max": round_stats.max },
        "games": games_json,
    });
    std::fs::create_dir_all(&out_dir).expect("create out dir");
    let artifact_path = out_dir.join("artifact.json");
    std::fs::write(
        &artifact_path,
        serde_json::to_vec_pretty(&artifact).unwrap(),
    )
    .expect("write artifact");
    println!("artifact: {}", artifact_path.display());

    if !all_ok || !budget_ok {
        std::process::exit(1);
    }
}

fn run_posture(
    lane: &mut BinLane,
    engine: &Engine,
    name: &str,
    seed0: u64,
    seeds: u64,
    cap: usize,
    hold: bool,
) -> (bool, Vec<serde_json::Value>) {
    let mut ok = true;
    let mut games: Vec<serde_json::Value> = Vec::new();
    for seed in seed0..seed0 + seeds {
        let (wire_stats, wire_decisions) =
            wire_play_game(lane, engine, seed, cap, Board::empty(), hold).expect("wire game");
        let (stats, decisions) = local_play_with_decisions(engine, seed, cap, Board::empty(), hold);
        let pass = wire_decisions == decisions && stats_eq(&wire_stats, &stats);
        if !pass {
            eprintln!(
                "G1 FAIL at posture {name} seed {seed}: decisions {}=={} stats {}",
                wire_decisions.len(),
                decisions.len(),
                stats_eq(&wire_stats, &stats)
            );
        }
        ok = ok && pass;
        games.push(serde_json::json!({
            "posture": name, "seed": seed, "cap": cap,
            "pieces": wire_stats.pieces, "lines": wire_stats.lines,
            "g1_pass": pass,
        }));
    }
    (ok, games)
}

fn resolve_bin() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    if let Some(p) = arg(&args, "--bin") {
        return PathBuf::from(p);
    }
    if let Some(p) = option_env!("CARGO_BIN_EXE_reflexer") {
        return PathBuf::from(p);
    }
    // `cargo run --example` does not build bin targets — resolve from the
    // conventional target dirs (honoring CARGO_TARGET_DIR), loud when absent.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("target"));
    for profile in ["release", "debug"] {
        let p = target_root.join(profile).join("reflexer");
        if p.exists() {
            return p;
        }
    }
    eprintln!(
        "reflexer bin not found — build it first:\n  cargo build --release --bin reflexer --example measure"
    );
    std::process::exit(1);
}

#[derive(PartialEq)]
enum Power {
    #[cfg_attr(not(any(target_os = "macos", target_os = "linux")), allow(dead_code))]
    Ac,
    Battery,
    NoBattery,
}

fn power_source() -> Power {
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("pmset")
            .arg("-g")
            .arg("batt")
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if text.contains("Battery Power") {
                return Power::Battery;
            }
            if text.contains("AC Power") {
                return Power::Ac;
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let mut battery_seen = false;
        if let Ok(dir) = std::fs::read_dir("/sys/class/power_supply") {
            for entry in dir.flatten() {
                if !entry.file_name().to_string_lossy().starts_with("BAT") {
                    continue;
                }
                battery_seen = true;
                let status =
                    std::fs::read_to_string(entry.path().join("status")).unwrap_or_default();
                if status.trim() == "Discharging" {
                    return Power::Battery;
                }
            }
        }
        if battery_seen {
            return Power::Ac;
        }
    }
    Power::NoBattery
}

fn loadavg() -> f64 {
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("sysctl")
            .arg("-n")
            .arg("vm.loadavg")
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            // "{ 3.42 3.90 4.33 } " → first figure
            let mut it = text
                .split_whitespace()
                .filter_map(|t| t.parse::<f64>().ok());
            if let Some(v) = it.next() {
                return v;
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = std::fs::read_to_string("/proc/loadavg") {
            if let Some(v) = text.split_whitespace().next().and_then(|t| t.parse().ok()) {
                return v;
            }
        }
    }
    -1.0
}

struct Stats {
    p50: u64,
    p90: u64,
    p99: u64,
    max: u64,
}

impl Stats {
    fn of(v: &[u64]) -> Self {
        let mut s = v.to_vec();
        s.sort_unstable();
        assert!(!s.is_empty(), "no samples");
        let at = |p: f64| s[((s.len() as f64 - 1.0) * p) as usize];
        Self {
            p50: at(0.50),
            p90: at(0.90),
            p99: at(0.99),
            max: *s.last().unwrap(),
        }
    }
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "p50={} p90={} p99={} max={}",
            self.p50, self.p90, self.p99, self.max
        )
    }
}
