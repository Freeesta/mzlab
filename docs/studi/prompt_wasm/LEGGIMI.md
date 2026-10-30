# Prompts of the study «latest WebAssembly features and toolchain»

Study: `docs/studi/WASM_ULTIME_FUNZIONI.md`. Each prompt is self-contained (copy it whole); one prompt per chat (Sonnet DEV).

**Build zone**: `tools/wasm_opt.sh`, `tools/build_feat_wasm.sh`, `tools/build_lettori_wasm.sh`, `.cargo/config.toml`, `Cargo.toml` `[profile]` sections, `rust-toolchain.toml` belong to ONE chat at a time; slot/NTS chats only measure with them. Other PRs that add a module line rebase on the build-zone PR in flight. Start a build-zone prompt only when fewer than 3 chats are running.

Order:
1. [W1](W1.md) `wasm-opt -O3` + binaryen 133 + relaxed-SIMD guard (build zone). [W4](W4.md) (WebKit on macOS, CI only) can run beside it.
2. [W2](W2.md) O3 on the parsing crates (build zone, after W1).
3. [W3](W3.md) `formula.ms1` as a lazy module: only after Federico's/the Coordinator's yes (decision 1 of the study), after W1.
4. [W5](W5.md) std sorts → `sortc` (size reserve; after W1, beside W3).
