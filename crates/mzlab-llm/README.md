# mzlab-llm

Language-model support for mzLab, kept apart from the engine:
- `tools`: the catalog `strumenti/1` of the tools an assistant may call, generated from the step registry (`tools::catalog_json`);
- `provider`: one interface (`llm-provider/1`) for Anthropic, OpenAI-compatible servers and Gemini: `request` builds `{url, headers, body}` without the key, `Stream` turns response bytes into normalized events, `http_error` maps failed statuses; `conv` and `event` are the types;
- `fake` (replay of recordings `llmreg/1`), `log` (`llmlog/1`, refuses API keys) and `cost` (estimate from the usage).

Pure Rust, no network, no key, no dependency beyond `mzlab-core`; it is not part of any `.wasm`. Used by `mzlab-cli llm`.
Notes for contributors: `docs/agenti/llm.md`.
