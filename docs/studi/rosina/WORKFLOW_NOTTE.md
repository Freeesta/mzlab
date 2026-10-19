# Rosina: workflow notturno del repository privato (proposta, da mettere in `Freeesta/mzlab-dati/.github/workflows/rosina.yml`)

Provato solo nella sintassi: il push nel privato richiede `add_repo`, che la chat di R-00 non ha ottenuto. Il Coordinatore (o la chat locale) lo copia nel privato con una PR e lo unisce.
Gira di notte e a mano: checkout del privato (i dati stanno in `dati/`, le verità in `scratchpad/rosina/verita/`) e di `Freeesta/mzlab` `main` (pubblico), poi i test `verita_*` del banco
(`crates/mzlab-lettori/tests/verita_<famiglia>.rs`). Ogni file produce una riga `file<TAB>L|P|NV|DIFETTO<TAB>livelli mancanti` in `MZLAB_ROSINA_RAPPORTO`; il riassunto formato × livello
è un artefatto, mai nel pubblico. I file oltre 90 MB non sono nel privato: li prova la chat locale con `MZLAB_DATI_GRANDI`. msconvert e `timsdata` non girano qui: le verità
arrivano già fatte da Windows/Mac (`coordinamento/dati_esterni/rosina/ORACOLI.md`).

```yaml
name: rosina
on:
  schedule: [{cron: "23 2 * * *"}]
  workflow_dispatch:
concurrency: {group: rosina, cancel-in-progress: true}
jobs:
  banco:
    runs-on: ubuntu-latest
    timeout-minutes: 90
    steps:
      - uses: actions/checkout@v4
        with: {path: privato}
      - uses: actions/checkout@v4
        with: {repository: Freeesta/mzlab, ref: main, path: mzlab}
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            mzlab/target
          key: rosina-${{ runner.os }}-${{ hashFiles('mzlab/Cargo.lock') }}
      - name: Truths and native files over 90 MB are not here
        run: |
          find privato/scratchpad/rosina/verita -name '*.verita.json' | wc -l
          echo "MZLAB_DATI=$GITHUB_WORKSPACE/privato" >> "$GITHUB_ENV"
          echo "MZLAB_ROSINA_RAPPORTO=$GITHUB_WORKSPACE/rapporto.tsv" >> "$GITHUB_ENV"
      - name: Bench
        working-directory: mzlab
        # a defect fails the step, the report is written either way
        run: cargo test --locked -p mzlab-lettori --features tutte --test 'verita_*' -- --nocapture
      - name: Summary format x level
        if: always()
        run: |
          [ -f rapporto.tsv ] || : > rapporto.tsv
          {
            echo "| file | esito | livelli mancanti |"; echo "|---|---|---|"
            awk -F'\t' '{printf "| %s | %s | %s |\n", $1, $2, $3}' rapporto.tsv
          } | tee -a "$GITHUB_STEP_SUMMARY" > riassunto.md
      - uses: actions/upload-artifact@v4
        if: always()
        with: {name: rosina-riassunto, path: "rapporto.tsv\nriassunto.md", retention-days: 30}
```
