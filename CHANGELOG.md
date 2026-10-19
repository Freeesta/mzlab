# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions: [SemVer](https://semver.org/) `MAJOR.MINOR.PATCH` (the first release is 1.0.0). Written in English only: the section of a tag becomes the text of its GitHub release (`tools/versione.py --nota`, `tools/note_release.py`). Sections: Added / Changed / Fixed.

Before putting the tag `vX.Y.Z`: move the entries of «Unreleased» below into the section `## [X.Y.Z] - YYYY-MM-DD`, bring the same version into `Cargo.toml`, `crates/mzlab-desktop/tauri.conf.json` and `CITATION.cff` (`python3 tools/versione.py --tag vX.Y.Z` checks it), then see `docs/agenti/rilascio.md`.

## [Unreleased]

### Changed
- Every entry of the right-click menu (spectrum, chromatogram, XIC, MRM, map) has its own icon on the left, in the colours of the theme.
- Right-click menu of spectrum, chromatogram, XIC and MRM: entries that do not apply right now stay in place, greyed out, instead of disappearing («Reset zoom», «Edit/Delete annotation», interval average, isotope profile, drawn formulas, MS² of the peak, MassBank/SIRIUS exports). Only entries that do not concern that data stay hidden (HR only, bench only, MS² only).
- «Reset zoom» is always the first entry, the annotations always the last.
- «Copy value» is also available in high-resolution spectra.
- Double click on a plot runs the same action as the menu: edit the annotation, otherwise reset the zoom, otherwise the spectrum at that RT (chromatogram) or the whole spectrum (spectrum).
- The right-click menu and the palette (⌘K) offer the panel layout: place next to, move up/down, full width, equal widths, reset.
- The «Drag here to place side by side» hint appears only once; new «Place side by side» step in the «Tools and moves» tour; balloons never overlap.
- New option in Settings «Spectrum next to the chromatogram» (off by default): the spectrum opened with a double click goes on the same row as the chromatogram when there is room; Ctrl+Z undoes it.

## [1.0.1] - 2026-10-14

Update of the desktop app and of the site.

### Added
- Desktop app: notice of a new version, read from the GitHub releases; the (i) box links «Download the desktop app».
- Desktop app: Tabui and the Pipeline panel with native computation (table, QC, export and report, file roles).
- RT×m/z map: axis lock with Shift, continuous-profile side spectrum and a tile pyramid computed in Rust.
- File list: click = active file, double click = only this one, Ctrl/Cmd+click and Shift+click for multiple selections.
- Default colours «Per file» (never grey or dashed without times).
- Agilent folders with a `.m` method are recognised, with a dedicated message for `.D`.
- Theory: chapter 23 on timing with a dynamic simulation, export to PDF, Word, Markdown and HTML.

### Changed
- Smoother loading: the first file of the list opens at once; the loading table moves from one field to the next with one click; the group bar spans the full width.
- Theory in English: it opens in Italian, with a notice.
- Theory: removed the second index, the objective boxes and the speech synthesis.
- Cold start of the desktop app behaves like a fresh install.

### Fixed
- Desktop app: the example mzML files also load on macOS, with the correct error message.
- A folder with a `.m` method is no longer ignored.

## [1.0.0] - 2026-10-14

First public release.

### Added
- mzLab in the browser (GitHub Pages) and as a desktop app: Data view per file for low and high resolution (chromatograms, spectra, RT×m/z map, MRM, integration, MS2 and libraries), Drawing (Ketcher), Theory and Practice; interface in Italian and English.
- Direct reading of Thermo `.raw`, SCIEX `.wiff` (with `.wiff.scan`) files and Agilent `.D` folders; other formats (Waters, Shimadzu, Bruker) are converted to mzML with MSConvert.
- Licences: code MIT, Theory and Practice texts CC BY 4.0, example files CC0, third-party material in `LICENZE-TERZI.md`.
- Accessibility (WCAG 2.2 AA) checked with axe on Data, HR bench, Drawing, Practice and the tour.
- Security policy (`SECURITY.md`), Dependabot, licence checks with `cargo-deny`, CycloneDX SBOM.
- Guides for students and teachers, privacy, accessibility and the hosting plan B in `docs/utenti/`.
- Release flow on the tag `vX.Y.Z`: site zip, installers for macOS, Windows and Linux, `SHA256SUMS`, SBOM.
