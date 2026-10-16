# Prompt delle chat DEV di Attributio (passaggio a Rust e binario MZ)

Restano i prompt dei pacchetti non ancora eseguiti (AT-04 → AT-07 → AT-08, MZ-05; stato in §6 dello studio); gli altri sono in `main`. Un file per pacchetto; ognuno si copia in una chat DEV (Sonnet) da sotto la riga «---». Lo studio è `docs/studi/ATTRIBUTIO.md`: §1 inventario dei moduli di Attributio, §3 luogo e soglie di parità, §4 Banco 2b, §5 binario MZ (mzmine 3.9.0), §6 piano, §8 domande Q1-Q7 con le scelte predefinite che i prompt seguono. Tutto si pubblica subito dietro la modalità amministratore (`uipAdminOn()`, `mzlab/web/settings.js:100`).

## Due binari
- **AT (priorità)**: Attributio 1.12.0 riscritto in Rust con parità sulla campagna sintetica (`tests/data/attributio/campagna_sintetica/`, riferimento `tools/attributio_riferimento.py`).
- **MZ (secondario)**: rilevamento alla mzmine 3.9.0 dai grezzi; MZ-05 lo collega alla cascata di AT come ingresso alternativo alla tabella di mzMine.

## Ordine e onde
| Onda | Pacchetti (in parallelo dentro l'onda) | Parte dopo il merge di |
|---|---|---|
| 1 | AT-00 fondamenta · MZ-00 diagnosi e dati d'oro di mzmine | — |
| 2 | AT-01 campagna e lettori · AT-05 abbinamento e librerie · MZ-01 ADAP · MZ-02 risolutore · MZ-03 join e riempimento | AT-00 (AT-01, AT-05); MZ-00 (MZ-01…03) |
| 3 | AT-02 cascata dei livelli · AT-06 spiegazioni e proposte · MZ-04 `.wasm` `untargeted` | AT-01 (AT-02); AT-01 e AT-05 (AT-06); MZ-01…03 (MZ-04) |
| 4 | AT-03 curazione e identità · MZ-05 isotopi, identità, ingresso della cascata | AT-02 (AT-03); MZ-04 e AT-01 (MZ-05) |
| 6 | AT-07 Banco 2b | AT-04, AT-06 |
| 7 | AT-08 esportazioni, quantificazione, gradiente | AT-07 |

Percorso critico: AT-00 → AT-01 → AT-02 → AT-03 → AT-04 → AT-07 → AT-08 (circa 10 giorni-chat su 14,5 del binario AT). Il binario MZ (circa 6,5) corre accanto e non lo rallenta.

## Chi tocca cosa
| Pacchetto | `.wasm` | File condivisi (chi unisce per secondo fa `git merge origin/main`) |
|---|---|---|
| AT-00 | nessuno | `AGENTS.md` §5 (una riga), `LICENZE-TERZI.md:16`, `docs/agenti/attributio.md` (con MZ-00) |
| AT-01, AT-02, AT-03, AT-05, AT-06 | nessuno (crate, feature `attributio`) | `crates/mzlab-feat/Cargo.toml` e `lib.rs` (feature `attributio`: la crea AT-01; AT-05 la crea se arriva prima), `crates/mzlab-feat/data/livelli-1.json` e `mzlab/web/livelli-1.js` (solo AT-02), `tools/attributio_riferimento.py` (uno stadio nuovo per pacchetto) |
| AT-04 → AT-07 → AT-08 | `attributio` (nuovo, in serie; AT-07 può aggiungere `attributio-rev`) | `tools/build_feat_wasm.sh`, `TP_Mine/files/feat-worker.js`, `feat-desktop-pool.js`, `crates/mzlab-desktop/src/feat_attributio.rs`, `TP_Mine/js/50…53_attributio*.js`, `tools/tipi/pagina.json` |
| MZ-00 | nessuno | `tools/valida/cancello_misure.py`, `tools/bench/` |
| MZ-01, MZ-02, MZ-03 | nessuno (crate, feature `untargeted`) | `crates/mzlab-feat/Cargo.toml` e `lib.rs`, `LICENZE-TERZI.md` (una riga di mzmine) |
| MZ-04 → MZ-05 | `untargeted` (nuovo, in serie) | `tools/build_feat_wasm.sh` (con AT-04), `TP_Mine/files/feat-worker.js` (con AT-04), `32/34/35_pipeline*.js`, `37_feature_table.js`, `29_untarget_identity.js`, `TP_Mine/data/cancello.json` |

Nessun pacchetto tocca il motore (`mzlab-core`, `mzlab-wasm`), `detect`, `proofs`, `mrm`, `finder`, `finder_msn`, `finder-lr`, `table`, `grafo`, `import`, né i file dei lavori in corso (RAW veloce, Tabui MS2 LR, formati nativi).

## Soglie (dettagli nei prompt)
- Binario AT, sulla sintetica: uguaglianza per livelli, sottolivelli, basi, nomi, identità, conteggi e classi di proposta; numeri 1e-9 relativo; frammenti assegnati con la stessa formula e ppm entro 0,005; frammentatore senza RDKit ≥ 0,90 di frammenti spiegati uguali. Le campagne vere le confronta l'agente locale sul Mac.
- Binario MZ: parità per stadio contro mzmine 3.9.0 ≥ 0,99 sui sintetici (`tests/data/untargeted/mzmine39/`), ≥ 0,97 sui dati veri del privato; cancello della ricetta `untargeted` (MZ-04, protocollo corretto di MZ-00): recall ≥ 0,95, precisione ≥ 0,95, Spearman ≥ 0,95, |log2| ≤ 0,10.
- Ogni `.wasm` < 512 000 B; il peso va nella descrizione della PR e nella riga del pacchetto in §6 dello studio.
