// @ts-strict
// The public demo of the assistant (profile «demo», settings.js): fixed settings that the visitor cannot change — the provider `mzlab-demo` (the proxy of mzLab, no key), level L1,
// file names hidden, a spending cap of 0.50 USD per panel — and one random session per opening of the panel (header `x-mzlab-session`). Loaded only after «Try the assistant».

export const DEMO_PROVIDER = "mzlab-demo";
/** the cap of the demo per opening of the panel, in USD (not editable) */
export const DEMO_SPEND_CAP = 0.5;

/** the script chips of the demo panel: the label (lang/ key) and the text sent, fixed in every language so that a strict recording replays it */
export const DEMO_CHIPS = [["llm.chip.explain", "Explain these files"], ["llm.chip.setup", "Set up mzFlow for me"]];

/** the words of the demo panel (lang/ keys): «Assistant not available», what still works, the spend counter and its one notice */
export const DEMO_WORDS = { unavailable: "llm.unavailable", rules: "llm.unavailable.rules", spend: "llm.spend", soft: "llm.spend.soft" };

/** @type {{provider: string, model: string, level: string, hideNames: boolean, spendCap: number, session: string} | null} */
let active = null;

/** a random session id (base64url of 18 bytes, 24 characters) */
export function newSession() {
  const b = crypto.getRandomValues(new Uint8Array(18));
  return btoa(String.fromCharCode(...b)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
/** turns the demo on for this page with the model of the provider list @param {string} model */
export function startDemo(model) { active = { provider: DEMO_PROVIDER, model, level: "L1", hideNames: true, spendCap: DEMO_SPEND_CAP, session: newSession() }; return active; }
export function demoActive() { return active; }
/** the cost in USD of `usage` [in, out] tokens with the price of a model (`price_per_mtok`), or null without one @param {number[]} usage @param {any} price */
export function spend(usage, price) { return price && price.in && price.out ? (usage[0] * price.in + usage[1] * price.out) / 1e6 : null; }
