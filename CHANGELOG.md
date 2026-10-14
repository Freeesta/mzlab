# Changelog

Formato [Keep a Changelog](https://keepachangelog.com/it/1.1.0/), versioni [SemVer](https://semver.org/lang/it/) `MAGGIORE.MINORE.PATCH` (la prima pubblicazione è la 1.0.0). In italiano, con un riassunto in inglese per ogni versione.
Format: Keep a Changelog; one release per laboratory semester, patches when needed. Release notes are taken from the section of the tag by `tools/versione.py --nota`.

Prima di mettere il tag `vX.Y.Z`: sposta qui sotto le voci di «Non rilasciato» nella sezione `## [X.Y.Z] - AAAA-MM-GG`, porta la stessa versione in `Cargo.toml`, `crates/mzlab-desktop/tauri.conf.json` e `CITATION.cff` (`python3 tools/versione.py --tag vX.Y.Z` lo controlla), poi vedi `docs/agenti/rilascio.md`.

## [Non rilasciato]

## [1.0.1] - 2026-10-14

Aggiornamento dell'app desktop e del sito.

### Aggiunto
- App desktop: avviso di nuova versione, letto dalle release di GitHub; nel riquadro (i) il link «Scarica l'app desktop».
- App desktop: Tabui e pannello Pipeline con calcolo nativo (tabella, QC, esportazione e rapporto, ruoli dei file).
- Mappa RT×m/z: blocco d'asse con Maiusc, spettro laterale a profilo continuo e piramide di tessere calcolata in Rust.
- Elenco file: clic = file attivo, doppio clic = solo questo, Ctrl/Cmd+clic e Maiusc+clic per selezioni multiple.
- Colori predefiniti «Per file» (mai grigio né tratteggio senza tempi).
- Cartelle Agilent con metodo `.m` riconosciute, con messaggio dedicato per le `.D`.
- Teoria: capitolo 23 sui tempi con simulazione dinamica, esportazione in PDF, Word, Markdown e HTML.

### Modificato
- Caricamento più fluido: si apre subito il primo file della lista; tabella di carico con un clic per passare da un campo all'altro; barra di gruppo a tutta larghezza.
- Teoria in inglese: si apre in italiano, con un avviso.
- Teoria: tolti il secondo indice, i box degli obiettivi e la sintesi vocale.
- Avvio a freddo dell'app desktop come nuova installazione.

### Corretto
- App desktop: i file di esempio mzML si caricano anche su macOS, con messaggio d'errore corretto.
- Cartella con metodo `.m` non più ignorata.

*English: desktop app notifies new releases from GitHub and links the download from the (i) box; Tabui and the Pipeline panel run natively in the app; RT×m/z map (axis lock, continuous side spectrum, Rust tile pyramid), file list selection (click, double click, Ctrl/Cmd, Shift), default "per file" colours, smoother loading, Agilent `.m` method folders, Theory chapter 23 and export; fixes: example mzML files load on macOS, cold start as a fresh install. Theory opens in Italian when the interface is English.*

## [1.0.0] - 2026-10-14

Prima pubblicazione.

### Aggiunto
- mzLab nel browser (GitHub Pages) e come app desktop: vista Dati per file a bassa e alta risoluzione (cromatogrammi, spettri, mappa RT×m/z, MRM, integrazione, MS2 e librerie), Disegno (Ketcher), Teoria e Pratica; interfaccia in italiano e inglese.
- Lettura diretta dei file Thermo `.raw`, SCIEX `.wiff` (con `.wiff.scan`) e delle cartelle Agilent `.D`; gli altri formati (Waters, Shimadzu, Bruker) si convertono in mzML con MSConvert.
- Licenze: codice MIT, testi di Teoria e Pratica CC BY 4.0, file d'esempio CC0, materiale di terzi in `LICENZE-TERZI.md`.
- Accessibilità (WCAG 2.2 AA) provata con axe su Dati, Banco HR, Disegno, Pratica e tour.
- Politica di sicurezza (`SECURITY.md`), Dependabot, controllo delle licenze con `cargo-deny`, SBOM CycloneDX.
- Guide per studenti e docenti, privacy, accessibilità e piano B dell'ospitalità in `docs/utenti/`.
- Flusso di rilascio sul tag `vX.Y.Z`: sito in zip, installatori per macOS, Windows e Linux, `SHA256SUMS`, SBOM.

*English: first public release. Browser site and desktop app for LC-MS data (low and high resolution), Drawing, Theory and Practice, in Italian and English; direct reading of Thermo `.raw`, SCIEX `.wiff` and Agilent `.D` (Waters, Shimadzu and Bruker: convert to mzML with MSConvert); code MIT, Theory and Practice texts CC BY 4.0, example files CC0; WCAG 2.2 AA checked with axe; security policy, licence checks, SBOM and the release workflow (site zip, installers, checksums).*
