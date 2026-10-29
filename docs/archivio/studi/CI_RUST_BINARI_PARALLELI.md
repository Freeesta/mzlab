# Proposta: binari di test Rust in parallelo nel job `rust-core`

Perché: `rust-core` è il job più lungo delle PR Rust (mediana 433 s, passo dei test 401 s, dati `stat-ci` dal 2026-10-22 14:00 UTC) e `cargo test` esegue i ~153 binari uno dopo l'altro; molti sono test di parità che lanciano un processo Python a thread singolo. In locale (4 CPU, profilo del workflow): compilazione 240 s a freddo, esecuzione in serie 377 s (60 s dopo la correzione di `number_replicates`), esecuzione dei 153 binari con `xargs -P4` **145 s e 152 s** (due giri, il secondo con `MZLAB_GOLDEN_STRICT=1 MZLAB_IDX_STRICT=1`), nessun fallimento né collisione sui file sintetici.

Misura in CI (job `rust-core` della PR dei campioni, 427 s): esecuzione dei 159 binari 201 s in serie (il runner è circa 1,9 volte più veloce del contenitore locale), il resto è compilazione e cache. Con quattro binari alla volta l'esecuzione scenderebbe a circa 80-100 s: guadagno atteso circa −100 s su `rust-core` (da ~430 a ~330 s, al livello di `rust-wasm` 325 s); il percorso critico delle PR Rust passa da ~8 a ~6,5 min. In `rust-misc-b` l'esecuzione misurata è 60 s su 268 (il resto è compilazione): non conviene.

## Modifica a `.github/workflows/rust.yml` (richiede il «via» di Federico)

Nel job `rust-core` sostituire il passo «Tests of mzlab-core, the readers and the libraries» con (stessi `env`):

```yaml
        run: |
          cargo test --locked --release -p mzlab-core -p mzlab-lettori -p mzlab-lib -p mzlab-libro -p mzlab-mol -p mzlab-target --doc
          cargo test --locked --release --no-run --message-format=json -p mzlab-core -p mzlab-lettori -p mzlab-lib -p mzlab-libro -p mzlab-mol -p mzlab-target 2>/dev/null \
            | python3 tools/ci_binari_test.py > bins.txt
          xargs -P4 -L1 python3 tools/ci_binari_test.py --esegui < bins.txt
```

e aggiungere `tools/ci_binari_test.py` (libreria standard): senza argomenti legge l'output JSON di cargo e stampa «cartella-crate eseguibile» per ogni artefatto di test; con `--esegui DIR EXE` esegue `EXE` con cwd e `CARGO_MANIFEST_DIR` = DIR, scrive l'output su stdout in un blocco unico e esce con il codice del binario (xargs esce 123 se uno fallisce). Va scritto con la sua prova (`tests/`) quando la proposta è accettata; non è stato aggiunto ora perché senza il workflow non sarebbe usato.

Rischi: test che scrivono lo stesso file fuori da una cartella temporanea (non trovati in due giri); output interleaved (risolto dal blocco unico); i doctest restano a parte con `--doc`.
