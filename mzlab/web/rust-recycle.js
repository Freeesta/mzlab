// @ts-strict
// Recycling of the worker of the Rust engine: the memory of a WASM instance is never given back, so a worker whose linear memory has grown past RECYCLE_MB and
// that has been idle for IDLE_MS (or whose page went to the background) is closed and a new one takes its place. The files of the session are opened again as after
// a fault, but nothing is counted as a fault and the page is not told. `seen(bytes)` is called with the memory the worker reports with each answer.
export const RECYCLE_MB = 192, IDLE_MS = 30000;

export function makeRecycler({ pending, worker, recycleNow, disabled }) {
  let mem = 0, timer = null;
  const now = () => { if (!worker() || pending.size) return; mem = 0; recycleNow(); };
  const check = force => {
    clearTimeout(timer); timer = null;
    if (!worker() || disabled() || mem <= RECYCLE_MB * 1048576 || pending.size) return;
    if (force || (typeof document !== "undefined" && document.hidden)) return now();
    timer = setTimeout(() => { if (!pending.size) now(); }, IDLE_MS);
  };
  if (typeof document !== "undefined") document.addEventListener("visibilitychange", () => { if (document.hidden) check(true); });
  return { seen(bytes) { if (typeof bytes === "number") { mem = bytes; check(false); } }, now };
}
