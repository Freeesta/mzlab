# Igiene di sicurezza del repository

- `dependabot.yml` (cargo, pip, npm di `tools/libro/editor`, github-actions: settimanale, un gruppo per ecosistema, massimo 3 PR aperte, mai unite da sole; `wasm-bindgen`, `js-sys`, `web-sys` senza salti major/minor, `tauri*` senza major) e `codeql.yml` (JavaScript e Rust, solo settimanale e a mano, non obbligatorio) non aggiungono job alle PR.
- Impostazioni che solo il proprietario accende a mano (nessun file le può impostare), in Settings → Code security: Dependabot alerts, Dependabot security updates, push protection, Private vulnerability reporting; in Settings → General: Allow auto-merge. La scansione dei segreti per i repository pubblici è gratuita e automatica (docs.github.com, «About secret scanning»); per gli altri interruttori la documentazione non dichiara un'attivazione predefinita, quindi vanno controllati.
- Niente CodeQL «default setup»: il flusso `codeql.yml` lo sostituisce (il default setup girerebbe anche sulle PR).
