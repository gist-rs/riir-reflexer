// vessel-mint — placeholder handler (secret-holder stub).
//
// Every route answers 503 until the autopilot lands: no minting exists in
// this deployment, by design. The only live fact is /healthz so monitors
// (and the keygen runbook) can distinguish "stub" from "gone".
//
// The genesis seed arrives as the VESSEL_MINT_SEED secret binding — this
// stub deliberately NEVER reads it, so there is no code path here that
// could log, echo, or exfiltrate seed material.

const BODY = JSON.stringify({
  error: "vessel-mint autopilot not deployed",
  code: "unconfigured",
  hint: "placeholder deployment holds the genesis secret only",
});

export default {
  async fetch() {
    return new Response(BODY, {
      status: 503,
      headers: { "content-type": "application/json" },
    });
  },
};
