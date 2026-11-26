Read docs/coda/BRIEF.md.

# MS2-LINK-FILES: Link MS2 spectra to rows, with provenance

**Goal.** An executor for the step `features.ms2_link` (today it has none): link DDA scans and dedicated MS2 files to rows by precursor m/z and RT window; each link carries file, SHA-256, scans, CE, apex RT, scan count, and the nitrogen-rule parity of the precursor; the user can link and unlink by hand (a decision, logged).

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 9 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §D.

**Read first.** `docs/studies/TABUI_MS2_LR.md`, `docs/agenti/data-channels.md` (`spectrum get --peak auto`), `spectrum dda-window`, `tp.link` (HR) code, `tests/test_parita_passi.py`.

**Already on main (checked 26 Nov, re-check before coding).** Net MS2 at the apex, DDA windows and `tp.link` pairing exist; dedicated files and manual links do not.

**Done means.** 1. Executor on site and native, byte test; windows as `--set` parameters with printed defaults. 2. Manual link and unlink as `ms2links/1` decisions (never silent). 3. Agent catalogs and `commands --find` test.

**Out of scope.** SIM traces as a slot (follow-up).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
