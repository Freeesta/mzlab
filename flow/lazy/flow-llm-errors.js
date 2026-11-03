// @ts-check
// Flow (administrator mode): plain-English texts of the errors that stop the assistant (provider, key, limits, level). Without them the panel would show the raw key.
// Extends F.message; loaded by flow-shell.js with flow-llm.js. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  /** @type {Record<string, (q: any) => string>} */
  const MSG = {
    "err.llm.auth": () => "The provider refused the key. Check it in Settings, Assistant.",
    "err.llm.rate": () => "The provider is busy or its limit is reached. Try again later.",
    "err.llm.context": () => "The conversation is too long for this model.",
    "err.llm.network": q => `The provider cannot be reached or the answer was cut off${q.reason ? ` (${q.reason})` : ""}.`,
    "err.llm.bad_response": () => "The provider answered with something the program cannot read.",
    "err.llm.aborted": () => "Stopped.",
    "err.llm.provider": () => "No provider is available here. Choose one in Settings, Assistant.",
    "err.llm.consent": () => "The consent for this provider is gone or the assistant is switched off: nothing was sent.",
    "err.llm.key.missing": () => "The provider's key is missing: set it in Settings, Assistant. Nothing was sent.",
    "err.llm.module": () => "The assistant module did not load: reload the page and try again.",
    "err.llm.limit.workspace": () => "The provider's workspace usage limit is reached: nothing more can be sent until it resets.",
    "err.llm.limit.spend": q => `A spending limit is reached${q.limit ? ` (${q.limit})` : ""}, or you chose not to go on: the assistant stops.`,
    "err.llm.proxy.closed": () => "The public demo of the assistant is closed for now: only its owner can use it, with the owner passphrase.",
    "err.llm.proxy.off": () => "The public demo of the assistant is switched off.",
    "err.llm.proxy.budget": q => `The budget of the public demo is used up${q.limit ? ` for this ${q.limit}` : ""}.`,
    "err.llm.proxy.body": q => `The relay of the demo refused the request${q.field ? ` (${q.field})` : ""}.`,
    "err.llm.proxy.origin": () => "The relay of the demo does not accept requests from this address.",
    "err.llm.limit.calls": () => "The assistant reached the maximum number of actions for one request.",
    "err.llm.plan.tries": () => "The assistant could not produce a valid plan after several tries.",
    "err.llm.profile.course": () => "The assistant is not available in this profile.",
    "err.llm.level.denied": () => "This action is not allowed at the chosen level.",
    "err.llm.context.budget": () => "The data to send do not fit in the context limit.",
  };
  const prev = F.message;
  F.message = (/** @type {any} */ p) => (MSG[p.key] ? MSG[p.key](p.params || {}) : prev(p));
})();
