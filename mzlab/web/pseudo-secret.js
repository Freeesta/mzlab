// @ts-check
// The secret of the pseudonyms of the anonymous exports (anon:<8 hex>): 32 random bytes made once, kept in this browser only
// (localStorage `qqq.pseudosecret`, never exported, never sent anywhere). Same value -> same pseudonym in every export of this browser.
const KEY = "qqq.pseudosecret";
/** @type {Uint8Array | null} */
let cached = null;

/** @returns {Uint8Array} */
export function pseudoSecret() {
  if (cached) return cached;
  try {
    const h = localStorage.getItem(KEY);
    if (h && /^[0-9a-f]{64}$/.test(h)) return (cached = Uint8Array.from(h.match(/../g) || [], x => parseInt(x, 16)));
  } catch (_) { /* storage blocked: the secret lives as long as the page */ }
  const b = crypto.getRandomValues(new Uint8Array(32));
  try { localStorage.setItem(KEY, Array.from(b, x => x.toString(16).padStart(2, "0")).join("")); } catch (_) { /* same */ }
  return (cached = b);
}
