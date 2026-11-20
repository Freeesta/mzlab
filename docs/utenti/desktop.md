# Installing the desktop app

The desktop app shows the same page as the site in a window, with a native calculation engine that opens large files faster. It is **not required**: the site works on its own. Files stay on your computer, as on the site.

## Where to get it
From the [Releases](https://github.com/Freeesta/mzlab/releases) page of the repository. If there is no release yet, use the site. Planned: macOS (`.dmg`, Apple Silicon and Intel) and Windows (`.exe`); Linux (AppImage and `.deb`) when published.

## Installing
- **Windows**: open the `.exe` and follow the steps. If "Windows protected your PC" (SmartScreen) appears, choose **More info → Run anyway**, only if the file comes from the Releases page of the official repository.
- **macOS**: open the `.dmg` and drag mzlab into Applications. If the app is not signed, on first launch hold Ctrl and click the icon, then **Open → Open**; alternatively System Settings → Privacy & Security → **Open Anyway**.
- Check that the file matches the `SHA256SUMS` of the release, if present.

## Updating
In the settings (gear icon) there is **"Check for updates"**: it downloads only the page files that changed, from the official site `https://freeesta.github.io/mzlab/`, and checks their SHA-256 hash. If a new app (native part) is needed, the message points to the Releases page. During lessons the [version freeze](teachers.md#version-freeze-during-lessons) applies.

## Problems
If the app does not start, try the site in a browser and report the problem with an [issue](https://github.com/Freeesta/mzlab/issues/new/choose) (operating system and version, no laboratory data). Best-effort support.
