# Release checklist vX.Y.Z

Copy to `CHECKLIST-vX.Y.Z.md` (same folder), tick every box with `[x]`: `tools/gate_release.py` refuses the tag while one is still open.

## Maps
- [ ] `python3 tools/indice.py --controlla` is green; `graphify-out` updated if present
- [ ] every area file of `docs/agenti/` is cited in §5 of `CLAUDE.md`

## Guides for agents
- [ ] every step of `mzlab-cli flow steps` has its row in `docs/agenti/cli.md`
- [ ] `docs/agenti/for-agents.md` and `llms.txt` (if it exists) cite only existing commands

## Guides for us
- [ ] `CLAUDE.md` and `docs/agenti/` describe the current state (no «now…», «it used to be…»)
- [ ] `docs/agenti/release.md` mirrors `release.yml`

## Readmes of everything
- [ ] every crate has a non-empty `README.md`
- [ ] the root `README.md` and `CITATION.cff` are up to date

## Versions
- [ ] `python3 tools/versione.py` is green (Cargo.toml, tauri.conf.json, CITATION.cff, `.zenodo.json`)

## Release notes
- [ ] `CHANGELOG.md` has the section `## [X.Y.Z]` in English, with no leftover «Unreleased»
- [ ] `python3 tools/versione.py --nota vX.Y.Z` prints the notes
