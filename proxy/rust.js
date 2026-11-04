// The Rust side of the proxy: the `mzlab-llm-wasm` module (wasm-bindgen --target web) bound once per isolate. The ledger, the prices and the closed
// answer schemas are Rust (`mzlab_llm::spend::Ledger`, `subcheck::closed_schemas`); this file only turns their JSON texts and errors into values.

function errorOf(e) {
  try {
    const v = JSON.parse(String(e));
    return { error_key: v.error_key || "err.llm.ledger", params: v.params || {} };
  } catch {
    return { error_key: "err.llm.ledger", params: {} };
  }
}

function wrap(f) {
  return (...args) => {
    try {
      return { value: f(...args) };
    } catch (e) {
      return { error: errorOf(e) };
    }
  };
}

/** `glue`: the module `mzlab_llm_wasm.js`; `module`: its `.wasm` as a `WebAssembly.Module` (Workers) or bytes (node). */
export function bindRust(glue, module) {
  glue.initSync({ module });
  return {
    rollover: wrap((state, day) => glue.ledger_rollover(state, day)),
    ipHash: wrap((state, ip, entropy) => JSON.parse(glue.ledger_ip_hash(state, ip, entropy))),
    admit: wrap((state, model, inBytes, maxTokens, keys, caps) =>
      JSON.parse(glue.ledger_admit(state, model, inBytes, maxTokens, JSON.stringify(keys), JSON.stringify(caps)))),
    settle: wrap((state, ticket, usage) =>
      JSON.parse(glue.ledger_settle(state, JSON.stringify(ticket), usage ? JSON.stringify(usage) : ""))),
    maxCost: wrap((model, estInput, maxTokens) => glue.max_cost(model, estInput, maxTokens)),
    schemas: () => JSON.parse(glue.assist_schemas()),
    Llm: glue.Llm,
  };
}
