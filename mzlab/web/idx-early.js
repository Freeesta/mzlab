// @ts-check
// @ts-strict
// The first TIC of an mzML that the index builder is about to read: the MS1 rt/tic of the scan table that `prepare_upload` made (the eight inputs of
// `build_mzidx`), as the page draws it before the index exists. Used by idx-worker.js, which prepares the table itself, and by rust-worker.js.

/**
 * @param {Uint8Array[]|null|undefined} inputs the eight inputs of the builder (levels, polarities, rt, tic, ...)
 * @returns {{ rt: number[], tic: number[], y: number[] }|null}
 */
export function earlyTicInputs(inputs) {
  if (!inputs || !inputs[2] || !inputs[3]) return null;
  const lv = inputs[0] ? new Uint8Array(inputs[0].buffer, inputs[0].byteOffset, inputs[0].byteLength) : null;
  const ra = new Float64Array(inputs[2].buffer, inputs[2].byteOffset, inputs[2].byteLength / 8);
  const ta = new Float64Array(inputs[3].buffer, inputs[3].byteOffset, inputs[3].byteLength / 8);
  if (!lv || !lv.some(x => x === 1)) return { rt: Array.from(ra), tic: Array.from(ta), y: Array.from(ta) };
  const rt = [], tic = [];
  for (let i = 0; i < lv.length; i++) if (lv[i] === 1) { rt.push(ra[i]); tic.push(ta[i]); }
  return { rt, tic, y: tic };
}
