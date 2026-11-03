// The proxy with every host service in memory, for the tests: the ledger state as a string behind one queue (as the Durable Object serializes it),
// a sliding-window rate limit, secrets and variables from plain objects, and the upstream fetch handed in (a fake provider).

import { makeProxy } from "../assist-proxy.js";
import { ledgerOps } from "../ledger-core.js";

export function memoryProxy({ rust, allow, upstream, secrets = {}, vars = { PHASE: "0" }, now = () => Date.now(), random, rate = { limit: 12, period: 60_000 }, caps }) {
  const rnd = random || ((n) => crypto.getRandomValues(new Uint8Array(n)));
  const ops = ledgerOps(rust, rnd);
  let state = "";
  let queue = Promise.resolve();
  // one operation at a time, each after a real await, so parallel requests interleave as they would against the Durable Object
  const serial = (f) => {
    const p = queue.then(async () => {
      await new Promise((r) => setTimeout(r, 0));
      const r = f(state);
      state = r.state;
      return r.error ? { error: r.error } : { value: r.value };
    });
    queue = p.catch(() => {});
    return p;
  };
  const ledger = {
    ipKey: (ip) => serial((s) => ops.ipKey(s, now(), ip)),
    admit: (a) => serial((s) => ops.admit(s, now(), a)),
    settle: (t, u) => serial((s) => ops.settle(s, now(), t, u)),
    alarm: () => serial((s) => ops.rollover(s, now())),
  };
  const hits = new Map();
  const rateLimit = async (key) => {
    const t = now();
    const recent = (hits.get(key) || []).filter((x) => t - x < rate.period);
    if (recent.length >= rate.limit) return false;
    recent.push(t);
    hits.set(key, recent);
    return true;
  };
  const proxy = makeProxy({ allow, ledger, secret: (n) => secrets[n], vars, rateLimit, upstream, caps });
  return { handle: proxy.handle, ledger, state: () => state };
}
