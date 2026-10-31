// @ts-strict
// Display form of a molecule name in the «Fragmentation graph»: first letter upper case. Only presentation: the stored name is never changed.

/** «timolol» → «Timolol». Names starting with a digit, a Greek letter, a bracket or an upper case letter, and names with a short lowercase prefix before a hyphen
 * (locants and stereo descriptors: «o-», «p-», «n-», «cis-», «tert-», «rac-»...), stay as they are. */
export function nomeMaiuscolo(/** @type {string} */ nome) {
  if (!/^[a-z]/.test(nome) || /^[a-z]{1,5}-/.test(nome)) return nome;
  return nome[0].toUpperCase() + nome.slice(1);
}
