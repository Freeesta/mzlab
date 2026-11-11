// Stand-in glue of a readers module for tests/lettori_trap_node.mjs: every evaluation is a new instance; probe and samples trap, fail or answer as globalThis.fakeMode says.
globalThis.fakeInstances = (globalThis.fakeInstances || 0) + 1;
const instance = globalThis.fakeInstances;
const answer = () => {
  if (globalThis.fakeMode === "trap") throw new WebAssembly.RuntimeError("unreachable");
  if (globalThis.fakeMode === "error") throw JSON.stringify({ error_key: "err.file.wiffLoad", params: {} });
  return JSON.stringify({ instance });
};
export default async () => {};
export const probe = answer;
export const open = () => 7;
export const samples = () => answer();
export const close = () => {};
