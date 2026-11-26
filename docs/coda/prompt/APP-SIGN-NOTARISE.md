Read docs/coda/BRIEF.md.

# APP-SIGN-NOTARISE: Sign and notarise the rolling desktop app

**Goal.** The app of `app-prebuilt` (`app-main.yml`) is unsigned: a Mac needs right-click Open or `xattr`, Windows shows SmartScreen. Sign the macOS app and notarise it (Apple Developer ID), and sign the Windows installer (SignPath or a certificate), in the same `desktop.yml` build, without changing the file names.

**Needs the owner.** An Apple Developer account, the certificate and notarisation credentials as secrets in an environment (never in the repo), and the choice of the Windows signing service. Do not start before the owner decides; no secret is ever written to a file or a log.

**Read first.** `docs/agenti/release.md` (Rolling desktop app of main, "Out of scope"), `.github/workflows/app-main.yml`, `.github/workflows/desktop.yml`, `docs/download-app.md`, `crates/mzlab-desktop/tauri.conf.json`.

**Done means.** (1) A signed and notarised `.dmg` from the same workflow, checked with `spctl` in the job. (2) `docs/download-app.md` and `tools/note_app_main.py` drop the unsigned steps for the signed platforms. (3) A run linked in the PR; `release.yml` uses the same signing.

**Out of scope.** Publishing a release or a version tag; a new updater.

**Follow-ups.** Closing step of the brief.
