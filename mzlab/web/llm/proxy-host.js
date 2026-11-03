// @ts-strict
// The ONE place of the host of the proxy of the public demo (provider `mzlab-demo`, proxy/): `https://<host>` with no path, or empty. Empty = nothing of the demo leaves the computer
// (the send stops with err.llm.network, reason «proxy»). tools/build_site.py checks the shape; the placeholder `https://proxy.invalid` of fornitori-1.json is replaced by Rust at send time.
export const PROXY_HOST = "";
