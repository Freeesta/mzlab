# mzlab-msnfigura-wasm

wasm-bindgen bridge of `mzlab_feat::msnfigure`: the SVG or stand-alone HTML figure of a fragmentation graph `msn-graph/1`, the bytes `flow run` writes for `msn.figure@1`.
A `.wasm` of its own, loaded by the Flow pool on the first `msn.figure` job. Errors are JSON `{"error_key","params"}` strings.
