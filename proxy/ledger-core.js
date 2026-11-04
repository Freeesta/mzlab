// The operations of the ledger on its JSON state, shared by the Durable Object (adapters/cloudflare.js) and the memory adapter of the tests. Each one
// starts the UTC day first (`rollover`), so a missed alarm never mixes two days. Synchronous: the host serializes the calls and stores the new state.

export function utcDay(ms) {
  return new Date(ms).toISOString().slice(0, 10);
}

/** `rust`: `bindRust(...)`; `random(n)`: n random bytes (the salt of the day is made from them inside Rust). Every result is `{state, value}` or `{state, error}`. */
export function ledgerOps(rust, random) {
  const day = (state, now) => {
    const r = rust.rollover(state, utcDay(now));
    return r.error ? { error: r.error } : { state: r.value };
  };
  const then = (state, now, f) => {
    const d = day(state, now);
    if (d.error) return { state, error: d.error };
    return f(d.state);
  };
  return {
    rollover: (state, now) => then(state, now, (s) => ({ state: s, value: true })),
    ipKey: (state, now, ip) =>
      then(state, now, (s) => {
        const r = rust.ipHash(s, ip, random(32));
        return r.error ? { state: s, error: r.error } : { state: JSON.stringify(r.value.state), value: r.value.ip_hash };
      }),
    admit: (state, now, a) =>
      then(state, now, (s) => {
        const r = rust.admit(s, a.model, a.inBytes, a.maxTokens, { session: a.session, ip_hash: a.ipHash || null }, a.caps);
        if (r.error) return { state: s, error: r.error };
        return { state: JSON.stringify(r.value.state), value: { ticket: r.value.ticket, session: r.value.session } };
      }),
    settle: (state, now, ticket, usage) =>
      then(state, now, (s) => {
        const r = rust.settle(s, ticket, usage);
        if (r.error) return { state: s, error: r.error };
        return { state: JSON.stringify(r.value.state), value: { charged: r.value.charged, session: r.value.session } };
      }),
  };
}
