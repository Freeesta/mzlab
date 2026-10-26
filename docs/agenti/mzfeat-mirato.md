# mzfeat: opzioni del passaggio mirato (`target`)

Spente di default: tabelle e `to_bytes()` identici. Il resto di `target` sta in `docs/agenti/mzfeat.md`.

- `TargetParams::estimate_points` (`Option<u16>`): un picco con meno di `min_points` ma almeno questi punti è accettato come «stimato» (`Cand::estimated`, bit `quality::REASON_ESTIMATED` in `qreason` della riga di `FilePeaks::peaks`); con meno punti è scartato, come sempre.
- `TargetParams::shape_tol_pct` (`Option<f64>`, controllo /\ in `target/extract.rs::rises_then_falls`): la traccia sale fino all'apice e scende dopo; un passo indietro fino a quella % dell'altezza è rumore, oltre il picco non è accettato (confronto col massimo corrente prima dell'apice e col minimo corrente dopo).
- `TargetParams::merge_overlap` (`target/merge.rs`): righe con m/z entro `ppm` e finestre di RT sovrapposte diventano una (la prima per massa); `TargetRow::merged` elenca le altre ipotesi, `TargetResult::name(reg, riga)` unisce le etichette con « / ». Per file vince la cella rilevata, poi la più grande; `found` conta le ipotesi trovate prima della fusione; il formato del `Registry` non cambia.
