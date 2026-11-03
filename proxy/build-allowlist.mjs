// The allow-list of the proxy, generated from the repository: the SHA-256 of every system prompt mzLab sends (with its role, models and the demo
// `max_tokens` cap of that role), of the phase prompt that may come mid-conversation, of the closed answer schemas (from Rust) and of the tool
// catalog as the Anthropic request carries it (built by Rust from `docs/schemi/strumenti-1.json`). Usage: node build-allowlist.mjs <wasm dir> > allowlist.json

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { sha256hex } from "./assist-proxy.js";
import { bindRust } from "./rust.js";

export const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const ASSIST = "crates/mzlab-llm/data/assist";
// the tool catalog the demo may send (P3 replaces it with the reduced demo catalog)
const CATALOGS = ["docs/schemi/strumenti-1.json"];
// the demo `max_tokens` per role (docs/studi/assistente-dati/09-worker.md §4): a role missing here stops the build
export const DEMO_MAX_TOKENS = { router: 200, metadati: 2000, cromatografia: 2000, parametri: 2000, interprete: 2000, grafici: 800, orchestrator: 1500, plan: 3000 };

const read = (root, p) => readFileSync(join(root, p), "utf-8");

/** Both forms of a prompt: as the file is and with the end trimmed (`Prompts::digest`). */
async function digests(text) {
  return [...new Set([await sha256hex(text), await sha256hex(text.trimEnd())])];
}

function cap(role) {
  const n = DEMO_MAX_TOKENS[role];
  if (!n) throw new Error(`no demo max_tokens for role ${role} (build-allowlist.mjs)`);
  return n;
}

export async function buildAllowlist(rust, root = ROOT) {
  const roles = JSON.parse(read(root, `${ASSIST}/assist-ruoli-1.json`));
  const system = {};
  const add = async (text, entry) => {
    for (const d of await digests(text)) system[d] = entry;
  };
  for (const r of roles.roles) await add(read(root, `${ASSIST}/${r.prompt}`), { role: r.role, models: [r.model], max_tokens: cap(r.role) });
  const o = roles.orchestrator;
  const orchestrator = { role: "orchestrator", models: [o.model], max_tokens: cap("orchestrator") };
  await add(read(root, `${ASSIST}/${o.prompt}`), orchestrator);
  await add(read(root, "crates/mzlab-llm/data/sistema-1.md"), orchestrator);
  const phase = {};
  for (const d of await digests(read(root, `${ASSIST}/${o.phase_prompt}`))) phase[d] = { max_tokens: cap("plan") };
  const schemas = [];
  for (const [, s] of rust.schemas()) schemas.push(await sha256hex(JSON.stringify(JSON.parse(s))));
  const tools = [];
  for (const c of CATALOGS) {
    const llm = new rust.Llm(read(root, c));
    const conv = { schema: "llm-conv/1", provider: "anthropic", model: o.model, system: "x", max_output_tokens: 1, messages: [{ role: "user", parts: [{ text: "x" }] }] };
    const req = JSON.parse(llm.request(JSON.stringify(conv)));
    const body = typeof req.body === "string" ? JSON.parse(req.body) : req.body;
    tools.push(await sha256hex(JSON.stringify(body.tools)));
    llm.free();
  }
  return { schema: "proxy-allow/1", system, phase, schemas, tools };
}

/** The bound Rust module of a wasm-bindgen `--target web` output folder. */
export async function loadRust(dir) {
  const glue = await import(pathToFileURL(join(dir, "mzlab_llm_wasm.js")).href);
  return bindRust(glue, readFileSync(join(dir, "mzlab_llm_wasm_bg.wasm")));
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const rust = await loadRust(process.argv[2] || join(ROOT, "proxy/wasm"));
  process.stdout.write(JSON.stringify(await buildAllowlist(rust), null, 1) + "\n");
}
