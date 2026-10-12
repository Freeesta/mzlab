# Changelog

Formato [Keep a Changelog](https://keepachangelog.com/it/1.1.0/), versioni [SemVer](https://semver.org/lang/it/) `0.MINORE.PATCH` fino all'articolo accettato. In italiano, con un riassunto in inglese per ogni versione.
Format: Keep a Changelog; one release per laboratory semester, patches when needed. Release notes are taken from the section of the tag by `tools/versione.py --nota`.

Prima di mettere il tag `vX.Y.Z`: sposta qui sotto le voci di «Non rilasciato» nella sezione `## [X.Y.Z] - AAAA-MM-GG`, porta la stessa versione in `Cargo.toml`, `crates/mzlab-desktop/tauri.conf.json` e `CITATION.cff` (`python3 tools/versione.py --tag vX.Y.Z` lo controlla), poi vedi `docs/agenti/rilascio.md`.

## [Non rilasciato]

### Aggiunto
- Politica di sicurezza (`SECURITY.md`), Dependabot, controllo delle licenze con `cargo-deny`, SBOM CycloneDX.
- Guide per studenti e docenti, privacy, accessibilità e piano B dell'ospitalità in `docs/utenti/`.
- Flusso di rilascio sul tag `vX.Y.Z`: sito in zip, installatori per macOS, Windows e Linux, `SHA256SUMS`, SBOM.

*English: security policy, dependency and licence checks, SBOM, user guides, and the release workflow (site zip, installers for macOS, Windows and Linux, checksums, SBOM).*
