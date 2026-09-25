// reflexer_host.mjs — load the reflexer engine wasm (crates/reflexer-wasm,
// wasm32-wasip1) and answer decision_wire request lines. Zero dependencies;
// the SAME file runs in the Cloudflare Worker (worker.mjs) and in the browser
// (reflex-site mirrors it as assets/reflexer_host.js for the arena's
// "Reflexer · wasm local" board), so the two hosts differ by the network hop
// and nothing else.
//
// WASI surface: the module imports clock_time_get, fd_write, environ_*,
// proc_exit and sched_yield. They are implemented here; anything a future
// build imports beyond that answers ENOSYS instead of failing to link.

const ERRNO_SUCCESS = 0;
const ERRNO_NOSYS = 52;

class WasiExit extends Error {
  constructor(code) {
    super(`reflexer wasm called proc_exit(${code})`);
    this.code = code;
  }
}

function wasiImports(getMemory, log) {
  const view = () => new DataView(getMemory().buffer);
  const now = globalThis.performance?.now ? () => globalThis.performance.now() : () => Date.now();
  const origin = Date.now();
  const pending = { 1: "", 2: "" };
  const dec = new TextDecoder();
  return {
    environ_sizes_get(countPtr, sizePtr) {
      const v = view();
      v.setUint32(countPtr, 0, true);
      v.setUint32(sizePtr, 0, true);
      return ERRNO_SUCCESS;
    },
    environ_get() {
      return ERRNO_SUCCESS;
    },
    clock_time_get(id, _precision, timePtr) {
      // id 0 = realtime, 1+ = monotonic/cpu. Workers freeze both clocks
      // during pure compute (they advance on I/O only), so an in-engine
      // duration measured inside a Worker reads ~0 — the Worker says so in
      // its X-Reflexer-Clock header; the round trip is the honest latency.
      const ms = id === 0 ? Date.now() : origin + now();
      view().setBigUint64(timePtr, BigInt(Math.round(ms * 1e6)), true);
      return ERRNO_SUCCESS;
    },
    fd_write(fd, iovs, iovsLen, nwrittenPtr) {
      const v = view();
      const mem = new Uint8Array(getMemory().buffer);
      let total = 0;
      for (let i = 0; i < iovsLen; i++) {
        const ptr = v.getUint32(iovs + i * 8, true);
        const len = v.getUint32(iovs + i * 8 + 4, true);
        if (fd === 1 || fd === 2) pending[fd] += dec.decode(mem.subarray(ptr, ptr + len));
        total += len;
      }
      for (const f of [1, 2]) {
        const lines = pending[f].split("\n");
        pending[f] = lines.pop();
        for (const line of lines) log(line);
      }
      v.setUint32(nwrittenPtr, total, true);
      return ERRNO_SUCCESS;
    },
    proc_exit(code) {
      throw new WasiExit(code);
    },
    sched_yield() {
      return ERRNO_SUCCESS;
    },
  };
}

function unpack(packed) {
  return [Number(packed >> 32n), Number(packed & 0xffffffffn)];
}

/** One live engine instance. After a trap the instance is poisoned — use
 * `Reflexer.instantiate` again (the Worker and the arena both do). */
export class Reflexer {
  static async instantiate(module, { log = (l) => console.error(`reflexer: ${l}`) } = {}) {
    let memory = null;
    const known = wasiImports(() => memory, log);
    const wasi = {};
    for (const imp of WebAssembly.Module.imports(module)) {
      if (imp.module !== "wasi_snapshot_preview1" || imp.kind !== "function") continue;
      wasi[imp.name] = known[imp.name] ?? (() => ERRNO_NOSYS);
    }
    const instance = await WebAssembly.instantiate(module, { wasi_snapshot_preview1: wasi });
    memory = instance.exports.memory;
    return new Reflexer(instance);
  }

  constructor(instance) {
    this.x = instance.exports;
    this.enc = new TextEncoder();
    this.dec = new TextDecoder();
    this.info = JSON.parse(this.take(this.x.rx_info()));
  }

  take(packed) {
    const [ptr, len] = unpack(packed);
    const text = this.dec.decode(new Uint8Array(this.x.memory.buffer, ptr, len));
    this.x.rx_free(ptr, len);
    return text;
  }

  /** One request line (a DecisionRequest JSON string) → one envelope JSON
   * string, byte-identical to the reflexer bin's stdout line. */
  decideLine(line) {
    const bytes = this.enc.encode(line);
    const ptr = this.x.rx_alloc(bytes.length);
    new Uint8Array(this.x.memory.buffer, ptr, bytes.length).set(bytes);
    return this.take(this.x.rx_handle(ptr, bytes.length));
  }

  /** decideLine over objects. */
  decide(request) {
    return JSON.parse(this.decideLine(JSON.stringify(request)));
  }
}

// ── the site's Tetris contract ─────────────────────────────────────────
// The arena plays hold-OFF (`hold_ready: false, held: null`), so the
// engine's canonical `place` enumeration is exactly the landing options in
// the site's own buildTurn order, labeled h0i<index> (katgpt-rs
// tetris_09_site_walk.rs asserts that equivalence for every recorded turn).

/** A `place` request for one site Tetris turn. `bag` = the pieces left in
 * the current 7-bag AFTER `next` was drawn, in draw order (empty = uniform). */
export function placeRequest({ board, cur, next, bag = [], nOptions }) {
  return {
    state: JSON.stringify({ board, cur, next, held: null, hold_ready: false, bag }),
    questions: [
      {
        id: "place",
        kind: "choice",
        prompt: "Which landing spot?",
        options: Array.from({ length: nOptions }, (_, i) => `h0i${i}`),
      },
    ],
  };
}

/** The chosen landing index from a `place` envelope, or null on
 * abstain / error (an abstain is a top-out: no legal spot). */
export function placePick(envelope) {
  const a = envelope?.response?.answers?.find((x) => x.question_id === "place");
  const idx = a?.outcome?.choice?.index;
  return Number.isInteger(idx) ? idx : null;
}
