// @ts-check
// Keeps the GitHub token of the admin editor: either sealed with a passphrase (AES-GCM, PBKDF2) in localStorage, or only for the tab (sessionStorage).
// Every storage access can throw (private window, blocked site data): the page must work without it.

const KEY = "mzlab.libro.token";
const ITER = 310000;

/** @param {Uint8Array} u */
const toB64 = (u) => btoa(String.fromCharCode(...u));
/** @param {string} s */
const fromB64 = (s) => Uint8Array.from(atob(s), (c) => c.charCodeAt(0));

/** @param {string} pass @param {Uint8Array<ArrayBuffer>} salt */
async function key(pass, salt) {
  const raw = await crypto.subtle.importKey("raw", new TextEncoder().encode(pass), "PBKDF2", false, ["deriveKey"]);
  return crypto.subtle.deriveKey({ name: "PBKDF2", salt, iterations: ITER, hash: "SHA-256" }, raw, { name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"]);
}

/** @param {() => any} f */
function safe(f) {
  try {
    return f();
  } catch (_) {
    return null;
  }
}

/** Seal the token with a passphrase and store it. Returns false when the browser refuses to store. @param {string} token @param {string} pass @param {string} [slot] */
export async function seal(token, pass, slot = KEY) {
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const ct = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv }, await key(pass, salt), new TextEncoder().encode(token)));
  const ok = safe(() => (localStorage.setItem(slot, JSON.stringify({ v: 1, salt: toB64(salt), iv: toB64(iv), ct: toB64(ct) })), true));
  return !!ok;
}

/** @param {string} [slot] */
export const hasSealed = (slot = KEY) => !!safe(() => localStorage.getItem(slot));

/** The sealed token, or null when the passphrase is wrong. @param {string} pass @param {string} [slot] */
export async function unseal(pass, slot = KEY) {
  const raw = safe(() => localStorage.getItem(slot));
  if (!raw) return null;
  try {
    const o = JSON.parse(raw);
    const pt = await crypto.subtle.decrypt({ name: "AES-GCM", iv: fromB64(o.iv) }, await key(pass, fromB64(o.salt)), fromB64(o.ct));
    return new TextDecoder().decode(pt);
  } catch (_) {
    return null;
  }
}

/** Keep the token for this tab only. @param {string} token @param {string} [slot] */
export const keepForTab = (token, slot = KEY) => !!safe(() => (sessionStorage.setItem(slot, token), true));
/** @param {string} [slot] */
export const tabToken = (slot = KEY) => safe(() => sessionStorage.getItem(slot));

/** @param {string} [slot] */
export function forget(slot = KEY) {
  safe(() => localStorage.removeItem(slot));
  safe(() => sessionStorage.removeItem(slot));
}
