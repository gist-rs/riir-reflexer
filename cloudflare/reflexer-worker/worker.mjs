// The reflexer Worker — the public engine over HTTP, stateless and free.
// The same reflexer.wasm the reflex-site arena runs in-tab ("Reflexer · wasm
// local"), served from Cloudflare's edge ("Reflexer · Cloudflare"), so the
// two boards differ by the network hop and nothing else.
//
//   GET  /            → {name, version, proto, genome, endpoints}
//   POST /v1/decide   → body: one decision_wire DecisionRequest (JSON)
//                       200: the success envelope (the bin's stdout line)
//                       422: the typed error envelope
//                       500: {proto, error:{code:"internal"}} (a trap)
//
// No secrets, no bindings, no storage, no logging of request bodies.
// CORS is open: the engine answers anyone, carries no credentials.
import wasmModule from "./reflexer.wasm";
import { Reflexer } from "./reflexer_host.mjs";

const MAX_BODY = 64 * 1024;
let engine = null;

const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET, POST, OPTIONS",
  "access-control-allow-headers": "content-type",
  "access-control-expose-headers": "x-reflexer-colo, x-reflexer-clock, x-reflexer-genome",
  "access-control-max-age": "86400",
};

function reply(status, body, colo, extra = {}) {
  return new Response(typeof body === "string" ? body : JSON.stringify(body), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "no-store",
      "x-reflexer-colo": colo ?? "unknown",
      ...CORS,
      ...extra,
    },
  });
}

async function live() {
  engine ??= await Reflexer.instantiate(wasmModule);
  return engine;
}

export default {
  async fetch(request) {
    const url = new URL(request.url);
    const colo = request.cf?.colo;
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: CORS });

    if (url.pathname === "/" || url.pathname === "/health") {
      const rx = await live();
      return reply(200, { ...rx.info, host: "cloudflare-worker", endpoints: { decide: "POST /v1/decide" } }, colo);
    }

    if (url.pathname !== "/v1/decide") return reply(404, { error: "not found", see: "GET /" }, colo);
    if (request.method !== "POST") return reply(405, { error: "POST a DecisionRequest" }, colo, { allow: "POST, OPTIONS" });

    const body = await request.text();
    if (body.length > MAX_BODY) return reply(413, { error: `body over ${MAX_BODY} bytes` }, colo);

    let line;
    try {
      line = (await live()).decideLine(body);
    } catch (e) {
      engine = null; // a trap poisons the instance — the next request re-instantiates
      return reply(500, { proto: 1, error: { code: "internal", message: String(e?.message ?? e), request_index: 1 } }, colo);
    }
    const ok = line.includes('"response":');
    return reply(ok ? 200 : 422, line, colo, {
      // Workers freeze the clock during pure compute, so the envelope's
      // in_engine_decision_ns reads ~0 here; the caller's round trip is the
      // honest latency (the arena capsule measures exactly that).
      "x-reflexer-clock": "frozen-during-compute",
      "x-reflexer-genome": engine?.info?.genome ?? "",
    });
  },
};
