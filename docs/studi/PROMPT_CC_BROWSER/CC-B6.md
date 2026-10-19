# CC-B6 (facoltativo) — Indici globali: IP, EA, elettrofilicità ω; BDE C–H solo se validata

Prompt per una chat DEV, da copiare dopo la riga «---». Parte solo con il sì di Federico. Studio: `docs/studi/CC_MOTORE_NEL_BROWSER.md` (§2E, §5, §7); file d'area: `docs/agenti/reattivita.md`.

---

## Stato
- Il motore `crates/mzlab-chem` calcola HOMO, LUMO e gap e le Fukui da tre SCC (N, N+1, N−1: `fukui.rs`).
- `reatt/1` ha già i campi `global.ip_ev`, `ea_ev`, `mu_ev`, `eta_ev`, `omega_ev`, ma `crates/mzlab-chem/src/reatt.rs:191` li scrive sempre `null`.
- La scheda «Info» (`TP_Mine/lazy/reattivita.js:131`) mostra già le righe IP, EA, μ, η, ω quando il campo è presente.
- Manca `crates/mzlab-chem/src/indici.rs` e il suo test.

## Da fare
1. **Rust** (`indici.rs`): IP = E(N−1) − E(N), EA = E(N) − E(N+1) dalle energie SCC già calcolate per le Fukui (nessun calcolo in più); correzione empirica di xtb `--vipea` se si trova la fonte, altrimenti valori non corretti con la nota «uncorrected». μ = −(IP+EA)/2, η = IP − EA, ω = μ²/(2η) (Parr 1999, doi:10.1021/ja983494x). Con il ripiego EHT i campi restano `null`. Riempi i cinque campi in `reatt.rs`.
2. **Prova di senso**: ω della serie N-fenilacrilammide, N-(p-metossifenil), N-(p-cianofenil) in ordine ciano > H > metossi; IP di benzene, fenolo e anilina contro il NIST Chemistry WebBook (con URL); scarto scritto in `docs/agenti/reattivita.md`.
3. **BDE C–H** solo se Federico la chiede dopo aver visto 1-2: E(R•) + E(H•) − E(RH) GFN2 a geometria rilassata; ≥ 15 legami con DOI; si mostra solo con Spearman ≥ 0,7, come «Prev.».
4. **Pagina**: nota fissa «Estimates from GFN2-xTB energy differences; compare molecules of the same family only»; valori anche nella tabella TDATI della scheda e nel JSON.

## Prove
`crates/mzlab-chem/tests/indici.rs` (formule a mano, ordine delle acrilammidi, IP); e2e `e2e_reattivita.py` esteso con le righe di «Info».

## Regole
Valgono AGENTS.md e i vincoli della famiglia CC: solo in modalità amministratore (`uipAdminOn()`), valori «Prev.» senza livello di Schymanski, niente Teoria né motore, `.wasm` `chem` sotto il suo tetto in `tools/build_feat_wasm.sh`, strutture di prova pubbliche, funzioni ≤ 80 righe, `cargo test --release -p mzlab-chem`, `python3 tools/verifica.py --cambiati`.
