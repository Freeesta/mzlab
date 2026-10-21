# Checklist di rilascio vX.Y.Z

Copia in `CHECKLIST-vX.Y.Z.md` (stessa cartella), spunta ogni casella con `[x]`: `tools/cancello_rilascio.py` rifiuta il tag finché ne resta una aperta.

## Mappe
- [ ] `python3 tools/indice.py --controlla` è verde; `graphify-out` aggiornato se presente
- [ ] ogni file d'area di `docs/agenti/` è citato nel §5 di `AGENTS.md`

## Guide per agenti
- [ ] ogni passo di `mzlab-cli steps` ha la sua riga in `docs/agenti/cli.md`
- [ ] `docs/agenti/per-agenti.md` e `llms.txt` (se esiste) citano solo comandi esistenti

## Guide per noi
- [ ] `AGENTS.md` e `docs/agenti/` descrivono lo stato attuale (niente «ora…», «prima era…»)
- [ ] `docs/agenti/rilascio.md` rispecchia `release.yml`

## Leggimi di tutto
- [ ] ogni crate ha un `README.md` non vuoto
- [ ] `README.md` della radice e `CITATION.cff` sono aggiornati

## Versioni
- [ ] `python3 tools/versione.py` è verde (Cargo.toml, tauri.conf.json, CITATION.cff, `.zenodo.json`)

## Note di rilascio
- [ ] `CHANGELOG.md` ha la sezione `## [X.Y.Z]` in inglese, senza «Unreleased» residuo
- [ ] `python3 tools/versione.py --nota vX.Y.Z` stampa le note
