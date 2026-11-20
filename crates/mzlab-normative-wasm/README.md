# mzlab-normative-wasm

wasm-bindgen bridge of the import of an official regulatory list (ECHA Candidate List, Proposition 65) into a private `.mzsus`, with the same bytes as `mzlab-cli db normative import-official`.
A `.wasm` of its own, loaded by the Library window only when the user imports a file. Errors are JSON `{"error_key","params"}` strings.
