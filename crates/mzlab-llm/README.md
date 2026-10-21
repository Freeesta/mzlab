# mzlab-llm

Language-model support for mzLab, kept apart from the engine. Today it builds the catalog `strumenti/1` of the tools an assistant may call, generated from the step registry (`tools::catalog_json`).
Pure Rust, no network, no key, no dependency beyond `mzlab-core`; it is not part of any `.wasm`. Used by `mzlab-cli llm tools`.
Notes for contributors: `docs/agenti/llm.md`.
