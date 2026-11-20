---
name: release
description: How to prepare an mzLab release up to the draft, consistent with docs/agenti/release.md. Never publishes; the owner presses Publish.
---
# Release preparation

Read `docs/agenti/release.md` first; it is the source of truth. If this skill and that file differ, follow the file and say so.

1. Copy `docs/rilascio/CHECKLIST-TEMPLATE.md` to `CHECKLIST-<version>.md` in the same folder and tick each box only after checking it.
2. Move the "Unreleased" entries of `CHANGELOG.md` into `## [X.Y.Z] - YYYY-MM-DD` (English; Added / Changed / Fixed; this text becomes the release notes). Preview: `python3 tools/versione.py --nota vX.Y.Z`.
3. Set the same `X.Y.Z` in `Cargo.toml`, `crates/mzlab-desktop/tauri.conf.json` and `CITATION.cff`; check with `python3 tools/versione.py` (and `--tag vX.Y.Z`). Version change touches the golden data: regenerate with `MZLAB_UPDATE_GOLDEN=1 python3 tools/verifica.py --solo <test>`.
4. `python3 tools/gate_release.py` must print OK (cli.md rows, INDICE citations, crate READMEs, code map, checklist all ticked).
5. DOI and `date-released` in `CITATION.cff` and `.zenodo.json` per `docs/zenodo.md`; the DOI line in `crates/mzlab-cli/src/citation.rs` goes together.
6. Merge the release PR through the Coordinator (`coordinator` skill). Then a dry run: `.github/workflows/release.yml` via `workflow_dispatch` on `main` with the version left empty builds everything and publishes nothing.
7. Real start: `workflow_dispatch` with the version typed twice (`versione` and `conferma`), or a `vX.Y.Z` tag pushed by the owner. The result is a **draft** release.
8. Never publish. The owner checks the draft, presses Publish and does the Zenodo and Software Heritage steps (`docs/agenti/release.md`).
