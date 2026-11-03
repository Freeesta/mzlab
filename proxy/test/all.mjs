// `node --test proxy/test/` on Node 22 loads this folder as a module (package.json "main"): it runs every *.test.mjs here in one process.
import { readdirSync } from "node:fs";

for (const f of readdirSync(new URL(".", import.meta.url)).filter((n) => n.endsWith(".test.mjs")).sort()) await import(new URL(f, import.meta.url));
