Read docs/coda/BRIEF.md.

# APP-ROLLING-NOTICE: Tell an installed app that a newer build of main exists

**Goal.** The installed desktop app notices only newer published releases (`src/release.rs`, `releases/latest`), and version 1.0 is not published, so an installed app never says that `app-prebuilt` has a newer native build. Add an opt-in notice: compare the commit of the running binary with the commit in the notes or a small `LATEST`-style asset of `app-prebuilt`, and show "A newer build of the app exists" with the download link.

**Read first.** `docs/agenti/rust-desktop.md` (New app version notice), `crates/mzlab-desktop/src/release.rs`, `tests/release.rs`, `mzlab/web/aggiorna.js`, `.github/workflows/app-main.yml`, `tools/note_app_main.py`.

**Done means.** (1) The workflow publishes the commit as a tiny asset (name fixed, listed in `tools/note_app_main.py`). (2) The Rust side asks once a day like the release notice, no user data, silent on any error; an off switch in Settings. (3) Tests with a local server, the `lang/` keys, the e2e step of `tests_e2e/e2e_aggiorna.py`.

**Out of scope.** Downloading or installing by itself (the binary does not update itself); changing what counts as a release.

**Follow-ups.** Closing step of the brief.
