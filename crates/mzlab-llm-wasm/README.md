# mzlab-llm-wasm

wasm-bindgen bridge of `mzlab-llm` for the assistant of the site: catalog and conversation in, request out; response bytes in, normalized events out; recordings `llmreg/1` for the `replay` transport.
No network and no key: the iframe `llm/trasporto.html` does the fetch. A `.wasm` of its own, loaded by `mzlab/web/llm/llm-worker.js` only after the user turned the assistant on.
