# Download the latest app

The desktop app built from the newest `main`, for Mac, Windows and Linux, is on one page that is replaced on every change:
<https://github.com/Freeesta/mzlab/releases/tag/app-prebuilt>

It is a pre-release called `app-prebuilt`: **not a release of mzLab** and never "latest" (version 1.0 is not published). Its notes say which commit and which date the files come from. The web part of an installed app also refreshes itself from the site; only the native part needs a new download.

Choose your file:
- **Mac** (Apple silicon and Intel): `mzLab-latest-Mac.dmg`
- **Windows**: `mzLab-latest-Windows.exe`
- **Linux**: `mzLab-latest-Linux.AppImage` or `mzLab-latest-Linux.deb`

The files are not signed or notarised yet, so the system warns the first time. Your files never leave your computer.

## Open the app on a Mac (unsigned)
1. Open the `.dmg` and drag mzLab into **Applications**.
2. In **Applications**, **right-click** (or Control-click) mzLab and choose **Open**, then **Open** again in the dialog. Do this once; afterwards it opens normally.
3. If macOS says the app "is damaged" or "cannot be opened", open **Terminal** and run: `xattr -dr com.apple.quarantine /Applications/mzlab.app`, then open the app again.
4. Another way: **System Settings > Privacy & Security**, scroll down, **Open Anyway** next to the message about mzLab.

## Windows and Linux
- Windows: if SmartScreen says "Windows protected your PC", click **More info**, then **Run anyway**.
- Linux: `chmod +x mzLab-latest-Linux.AppImage`, then run it; or install the `.deb` with your package manager.

## Check the download (optional)
`SHA256SUMS` on the same page lists the checksum of each file: `shasum -a 256 -c SHA256SUMS` on a Mac or Linux.

## For maintainers
Built by `.github/workflows/app-main.yml`; see `docs/agenti/release.md`. Apple signing and notarisation are the owner's later decision.
