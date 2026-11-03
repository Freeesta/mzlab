// The allow-list of the proxy, generated from the repository: the SHA-256 of every system prompt mzLab sends (with its role, models and the demo
// `max_tokens` cap of that role, `demo_max_tokens` of assist-ruoli-1.json), of the phase prompt that may come mid-conversation, of the closed answer
// schemas (from Rust) and of the demo tool catalog as the Anthropic request carries it (built by Rust from `docs/schemi/strumenti-demo-1.json`).
// Usage: node build-allowlist.mjs <wasm dir> > allowlist.json

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { sha256hex } from "./assist-proxy.js";
import { bindRust } from "./rust.js";

export const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const ASSIST = "crates/mzlab-llm/data/assist";
// the tool catalog the demo sends (the subset `mzlab_llm::tools::DEMO_TOOLS`)
export const CATALOGS = ["docs/schemi/strumenti-demo-1.json"];

const read = (root, p) => readFileSync(join(root, p), "utf-8");

/** Both forms of a prompt: as the file is and with the end trimmed (`Prompts::digest`). */
async function digests(text) {
  return [...new Set([await sha256hex(text), await sha256hex(text.trimEnd())])];
}

/** The demo `max_tokens` of a role (or of the orchestrator's `demo_max_tokens_plan`): a missing one stops the build. @param {any} entry @param {string} key */
function cap(entry, key = "demo_max_tokens") {
  const n = entry[key];
  if (!Number.isInteger(n) || n < 1) throw new Error(`no ${key} for ${entry.role || "orchestrator"} (assist-ruoli-1.json)`);
  return n;
}

/** The demo caps per role as the allow-list holds them, read from assist-ruoli-1.json. */
export function demoMaxTokens(root = ROOT) {
  const roles = JSON.parse(read(root, `${ASSIST}/assist-ruoli-1.json`));
  const out = Object.fromEntries(roles.roles.map((r) => [r.role, cap(r)]));
  return { ...out, orchestrator: cap(roles.orchestrator), plan: cap(roles.orchestrator, "demo_max_tokens_plan") };
}

export async function buildAllowlist(rust, root = ROOT) {
  const roles = JSON.parse(read(root, `${ASSIST}/assist-ruoli-1.json`));
  const system = {};
  const add = async (text, entry) => {
    for (const d of await digests(text)) system[d] = entry;
  };
  for (const r of roles.roles) await add(read(root, `${ASSIST}/${r.prompt}`), { role: r.role, models: [r.model], max_tokens: cap(r) });
  const o = roles.orchestrator;
  const orchestrator = { role: "orchestrator", models: [o.model], max_tokens: cap(o) };
  await add(read(root, `${ASSIST}/${o.prompt}`), orchestrator);
  await add(read(root, "crates/mzlab-llm/data/sistema-1.md"), orchestrator);
  const phase = {};
  for (const d of await digests(read(root, `${ASSIST}/${o.phase_prompt}`))) phase[d] = { max_tokens: cap(o, "demo_max_tokens_plan") };
  const schemas = [];
  for (const [, s] of rust.schemas()) schemas.push(await sha256hex(JSON.stringify(JSON.parse(s))));
  const tools = [];
  for (const c of CATALOGS) {
    const llm = new rust.Llm(read(root, c));
    const conv = { schema: "llm-conv/1", provider: "mzlab-demo", model: o.model, system: "x", max_output_tokens: 1, messages: [{ role: "user", parts: [{ text: "x" }] }] };
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
