# Prove di mutazione (cargo-mutants) dei lettori e del modulo wiff

Che cosa dicono i giri di `cargo-mutants` su `crates/mzlab-lettori` e su `crates/mzlab-core/src/wiff/`, quali sopravvissuti restano e perché. Sono giri a campione: il giro intero (3891 mutanti nei lettori, 1582 nel modulo wiff) chiede giorni con le suite complete.

## Come si rifà
- `cargo install cargo-mutants` (versione 27). Il modulo wiff: `cargo mutants -p mzlab-core -f 'crates/mzlab-core/src/wiff/*.rs' --shard 0/16 --sharding round-robin -j 3 -o DIR -- --lib --test 'wiff_*' --test mzml_modes_sciex` (un `--test` per file `tests/wiff_*.rs`; la suite intera di `mzlab-core` costa ~18 min per mutante). Un mutante solo: `-F '<riga di missed.txt>'`.
- I lettori: `cargo mutants -p mzlab-lettori --shard 0/32 --sharding round-robin -j 3 -o DIR` (tutti i test del crate, ~3 min per mutante).
- Il codice sotto `#[cfg(feature = "estesi")]` (`extended.rs`, `scan_extended.rs`, `run_extended.rs`) si compila solo con la feature: un giro con le feature di default di `mzlab-core` lo dà sempre «missed». Va rifatto con `--features sciex`; la CI lo prova perché compila `mzlab-core` insieme a `mzlab-lettori` (`ci_binari_test.py -p mzlab-core -p mzlab-lettori`, feature unificate).
- Le cartelle `mutants.out` non vanno nel repository; ogni giro chiede ~4 GB di disco per job.

## Modulo wiff (campione 1/16, 99 mutanti)
- Primo giro: 53 presi, 28 sopravvissuti, 17 non compilabili, 1 attesa scaduta.
- Presi poi dai test nuovi: passo DAD e asse delle lunghezze d'onda (`wiff_synthetic_ci.rs`, passo di 2 nm perché con 1 nm `*` e `/` coincidono), resto dei record DDE, scostamento ITC e ogni gettone TDC (`dad_tests.rs`), nomi `.dam` (`method_name.rs`).
- Presi anche, dai test nuovi: termine CV del tipo di spettro e polarità dalla tensione IS (`modes.rs`, `source_polarity`), intestazione mzML (`run.rs`: ogni campo, modello noto e ignoto), dimensione del blocco e tempo di una sezione non indicizzata (`unindexed.rs`, anche con due esperimenti per ciclo), corse di zeri di un blocco MRM sparso (`scan_extended.rs`).
- Restano, solo con dati reali (i test che li coprono leggono i `.wiff` del repository privato e senza di esso si saltano): `reader.rs` `wiff_member` → `Some(0)` (nei file sintetici il `.wiff` è sempre il primo membro: equivalente lì), `scan_header`, `add_unindexed` (`==` → `!=`); `reader_extra.rs` `header_tic` e il campo `tic` di `extra_of` (servono spettri a scansione con TIC nell'indice).
- Restano per scelta: `reader.rs` `not_carried` (solo testo descrittivo, nessun comportamento), `dad_wavelength` (passa soltanto a `dad_uv::dad_wavelength_json`, provata da `mzml_dad_tests.rs`; nessun test apre il lettore wiff su un file con DAD), `scan.rs:171` `>` → `==` nel tetto di memoria (servirebbe un blocco di ~13 MB per toccarlo), `scan.rs:38` `+=` → `-=` in `Nibbles::long` (ciclo senza fine: attesa scaduta, cioè preso).
- Sotto `estesi`, da rifare con `--features sciex`: `extended.rs` `refine`, `>` → `>=` in `refine`, `header_estesi`; `run_extended.rs` `visit`, `||` → `&&` in `chromatograms`.
