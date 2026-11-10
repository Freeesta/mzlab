// Stand-in glue of a readers module for tests/lettori_trap_node.mjs: every evaluation is a new instance; probe traps, fails or answers as globalThis.fakeMode says.
globalThis.fakeInstances = (globalThis.fakeInstances || 0) + 1;
const instance = globalThis.fakeInstances;
export default async () => {};
export const probe = () => {
  if (globalThis.fakeMode === "trap") throw new WebAssembly.RuntimeError("unreachable");
  if (globalThis.fakeMode === "error") throw JSON.stringify({ error_key: "err.file.wiffLoad", params: {} });
  return JSON.stringify({ instance });
};
