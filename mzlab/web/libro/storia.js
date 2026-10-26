// @ts-strict
// Undo/redo of the Teoria editor and the draft kept in the browser. The history lives in memory per chapter; the draft (chapter text and summary, never the token)
// goes to localStorage so a closed tab does not lose the work. Every storage access can throw (private window, blocked site data): the editor must work without it.

/**
 * Linear history of the text of one chapter: record() after each change, undo() and redo() walk it.
 * @param {string} first the text as loaded @param {number} [limit] states kept
 */
export function makeHistory(first, limit = 200) {
  /** @type {string[]} */
  let states = [first];
  let at = 0;
  return {
    /** A new state; the same text as the current one is not a step, and a step after an undo drops the redo states. @param {string} text */
    record(text) {
      if (text === states[at]) return false;
      states = states.slice(0, at + 1);
      states.push(text);
      if (states.length > limit) states.shift();
      at = states.length - 1;
      return true;
    },
    canUndo: () => at > 0,
    canRedo: () => at < states.length - 1,
    /** The previous text, or null at the start. */
    undo() {
      return at > 0 ? states[--at] : null;
    },
    /** The next text, or null at the end. */
    redo() {
      return at < states.length - 1 ? states[++at] : null;
    },
    /** The text now. */
    current: () => states[at],
  };
}

const PREFIX = "mzlab.libro.teoria.draft.";

/** @param {() => any} f */
function safe(f) {
  try {
    return f();
  } catch (_) {
    return null;
  }
}

/** @typedef {{text: string, base: string, summary: string, saved: string}} Draft */

/** Keep the draft of a chapter. `base` is the blob sha of the chapter the draft starts from. @param {string} name @param {Draft} d @param {Storage} [store] */
export function saveDraft(name, d, store) {
  return !!safe(() => ((store || localStorage).setItem(PREFIX + name, JSON.stringify({ text: d.text, base: d.base, summary: d.summary, saved: d.saved })), true));
}

/** The draft of a chapter, or null. @param {string} name @param {Storage} [store] @returns {Draft | null} */
export function loadDraft(name, store) {
  const raw = safe(() => (store || localStorage).getItem(PREFIX + name));
  if (!raw) return null;
  try {
    const o = JSON.parse(raw);
    return typeof o.text === "string" && typeof o.base === "string" ? { text: o.text, base: o.base, summary: String(o.summary || ""), saved: String(o.saved || "") } : null;
  } catch (_) {
    return null;
  }
}

/** @param {string} name @param {Storage} [store] */
export function clearDraft(name, store) {
  safe(() => (store || localStorage).removeItem(PREFIX + name));
}
