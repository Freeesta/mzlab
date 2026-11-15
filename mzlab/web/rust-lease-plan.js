// @ts-check
// @ts-strict
// Which read leases (archivio.js) a request of the engine worker needs awake. Used by rust-worker.js before every request: a request
// that reads no leased index wakes nothing, one that reads a single file wakes that file's lease, anything unknown wakes them all.

/** ops that read no leased file: they never call the engine on an open index, `lrOpen` or `ARCH.lease` (tests/test_lease_plan.py greps their case lines) */
export const READ_FREE = new Set([
  "lr_campioni", "lr_kv_get", "lr_kv_set", "lr_stat", "lr_flow_command", "lr_flow_line", "lr_flow_sched", "lr_molecules", "lr_molecole_archivio",
  "lr_composition", "lr_isotope_fine", "lr_provenance", "lr_ricerca", "lr_adduct_table", "lr_formula", "lr_ion_pattern", "lr_cancel",
  "open", "close", "reset", "down", "budget", "natsamples", "natsame", "nat2mzml", "natregistro", "natsrm", "mzml2cen",
]);

/** ops that read the index of ONE file: the one at `m.slot` (an lr_ op names its file) */
const OF_SLOT = new Set(["tic", "xic", "spectra", "route", "idxxic", "idxxicbin", "scaninfo", "mzmlroute", "scanbin", "specexact", "mrm", "mrmchrom", "fileinfo", "masscal", "canali"]);
const OF_NAME = new Set(["lr_entry", "lr_items", "idxopen"]);

/**
 * @param {{ op: string, name?: string, slot?: number }} m
 * @param {(slot: number) => string | undefined} nameOfSlot index file name of a slot; "" = a slot with no lease (an mzML opened as a Blob); undefined = unknown
 * @returns {string[] | null} names of the indexes to wake ([] = none); null = every lease (the safe default)
 */
export function leasePlan(m, nameOfSlot) {
  if (READ_FREE.has(m.op)) return [];
  if (OF_NAME.has(m.op) && typeof m.name === "string") return [m.name];
  if (OF_SLOT.has(m.op) && typeof m.slot === "number") { const n = nameOfSlot(m.slot); return n === undefined ? null : n ? [n] : []; }
  return null;
}
