// The engine start (issue #3509): a worker that dies while lr_boot is pending must not end the start as "engine lost" when the supervisor has
// already restarted it (E1); a second loss still fails, flagged dead (E2); the recovery drops the app caches and reloads once, never twice (E3).
import assert from "node:assert/strict";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const workers = [];
class FakeWorker {
  constructor(url) { this.log = []; if (String(url).endsWith("rust-worker.js")) workers.push(this); }
  postMessage(m) { this.log.push(m); if (m.op && m.id && this.auto) this.auto(m, this); }
  terminate() {}
  reply(m) { this.onmessage({ data: m }); }
}
Object.assign(globalThis, { Worker: FakeWorker, window: globalThis, self: globalThis, MZLAB_DESKTOP: true, fetch: () => Promise.reject(new Error("no net")) });
const { createBridge } = await import(pathToFileURL(path.join(root, "mzlab/web/rust-bridge.js")).href);
const { createLr } = await import(pathToFileURL(path.join(root, "mzlab/web/lr.js")).href);
const { recoverEngine, recoverDone } = await import(pathToFileURL(path.join(root, "mzlab/web/engine-recupera.js")).href);

const sleep = ms => new Promise(r => setTimeout(r, ms));
const until = async f => { for (let i = 0; i < 400 && !f(); i++) await sleep(5); assert.ok(f(), "timeout"); };
const answer = (m, w) => w.reply({ id: m.id, op: m.op, info: m.op === "lr_boot" ? "[]" : "{}" });
const make = () => { workers.length = 0; const b = createBridge({ workerUrl: new URL("file:///x/rust-worker.js"), notify: () => {} }); b.start(); return { b, lr: createLr({ rb: b }) }; };
const boots = w => w.log.filter(m => m.op === "lr_boot").length;
const quiet = console.warn; console.warn = () => {};

// E1: the first worker dies with lr_boot pending; its replacement answers it
{
  const { lr } = make();
  const first = workers[0];
  const done = lr.boot(true);
  await until(() => boots(first) === 1);
  workers[0].onerror({ message: "boom" });
  await until(() => workers.length === 2);
  workers[1].auto = answer;
  await until(() => boots(workers[1]) === 1);
  await done;                                                                   // resolves: the boot ran on the replacement
}
// E2: the replacement dies too: the start fails and says the engine died (the page then purges its cache)
{
  const { lr } = make();
  const done = lr.boot(true).then(() => null, e => e);
  await until(() => boots(workers[0]) === 1);
  workers[0].onerror({ message: "boom" });
  await until(() => workers.length === 2 && boots(workers[1]) === 1);
  workers[1].onerror({ message: "boom" });
  const e = await done;
  assert.ok(e && e.dead, "the second loss is reported as a dead engine");
}
console.warn = quiet;
// E3: recovery drops only the app caches, unregisters the workers, reloads once
{
  const store = new Map(), st = { getItem: k => store.get(k) ?? null, setItem: (k, v) => store.set(k, v), removeItem: k => store.delete(k) };
  const caches = new Map([["mzlab-app-abc", 1], ["mzlab-big-x", 1], ["other", 1]]);
  const cacheApi = { keys: async () => [...caches.keys()], delete: async k => caches.delete(k) };
  let unreg = 0, reloads = 0;
  const sw = { getRegistrations: async () => [{ unregister: async () => { unreg++; return true; } }] };
  const opt = { log: () => {}, reload: () => { reloads++; }, prefix: "mzlab-app-", storage: st, cacheApi, sw };
  assert.equal(await recoverEngine(opt), "reloading");
  assert.deepEqual([...caches.keys()], ["mzlab-big-x", "other"]);
  assert.equal(unreg, 1); assert.equal(reloads, 1);
  assert.equal(await recoverEngine(opt), "already");                            // the second failure: no loop
  assert.equal(reloads, 1);
  recoverDone(st);
  assert.equal(await recoverEngine(opt), "reloading");                          // an engine that came up clears the flag
}
console.log("ok");
