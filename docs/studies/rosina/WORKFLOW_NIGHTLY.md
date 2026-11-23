# Rosina: nightly workflow of the private repo (proposal, to place in `Freeesta/mzlab-dati/.github/workflows/rosina.yml`)

Syntax-checked only: pushing to the private repo needs `add_repo`, which the R-00 chat did not obtain. The Coordinator (or the local chat) copies it into the private repo with a PR and merges it.
It runs at night and by hand: checkout of the private repo (data in `dati/`, truths in `scratchpad/rosina/verita/`) and of `Freeesta/mzlab` `main` (public), then the `truth_*` tests of the bench
(`crates/mzlab-lettori/tests/truth_<family>.rs`). Each file produces a line `file<TAB>L|P|NV|DEFECT<TAB>missing levels` in `MZLAB_ROSINA_RAPPORTO`; the format × level summary
is an artifact, never in the public repo. Files over 90 MB are not in the private repo: the local chat tests them with `MZLAB_DATI_GRANDI`. msconvert and `timsdata` do not run here: the truths
arrive already made from Windows/Mac (`coordinamento/dati_esterni/rosina/ORACOLI.md`).

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
        run: cargo test --locked -p mzlab-lettori --features tutte --test 'truth_*' -- --nocapture
      - name: Summary format x level
        if: always()
        run: |
          [ -f rapporto.tsv ] || : > rapporto.tsv
          {
            echo "| file | outcome | missing levels |"; echo "|---|---|---|"
            awk -F'\t' '{printf "| %s | %s | %s |\n", $1, $2, $3}' rapporto.tsv
          } | tee -a "$GITHUB_STEP_SUMMARY" > riassunto.md
      - uses: actions/upload-artifact@v4
        if: always()
        with: {name: rosina-riassunto, path: "rapporto.tsv\nriassunto.md", retention-days: 30}
```
