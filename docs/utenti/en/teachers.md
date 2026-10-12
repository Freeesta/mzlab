# Guide for teachers

## In short
- mzLab is a website: students open <https://freeesta.github.io/mzlab/> on a computer or tablet, with nothing to install and no account. A [desktop app](desktop.md) with the same look is also available.
- The program **does not give the answers** (no list of TPs, no calibration lines or report tables): it shows the data, and students reason and calculate in Excel. At unit resolution an m/z is a candidate, not an identification.
- Theory and Practice are in Italian only; the Data and Drawing interface is in Italian and English.
- The first visit downloads the calculation engine, so a network connection is needed (in class, have students open the site before the lesson).

## Before the lesson
1. Try the site with the **example files** ("Try the example files") and with your own data.
2. Ask students to open the site once beforehand: after the first load the browser keeps most of it.
3. `.wiff` files must be converted to `.mzML` (MSConvert, profile mode): the guide is on the loading screen.
4. **Student and laboratory data are never published**: they do not belong in issues, forums or public repositories.

## Version freeze during lessons
The site is updated at every approved change. So that everyone sees the same program in class, this rule applies:

- **During teaching weeks** (the dates are in the course calendar kept by the maintainers) the version is **frozen**: only bug fixes are published, no new features and no interface changes.
- **Outside lessons** publishing is normal; a minor version is released before each laboratory semester.
- Teachers tell the maintainers the dates of their courses (with a "question" issue) **at least two weeks ahead**, so that the freeze covers them.
- The site always shows the latest published version. If you need to say exactly which version you used (for an exam, for instance), note the date on which you used it and cite it together with the software.

## If a problem blocks the lesson
Reloading the page (hard reload: Ctrl+Shift+R) fixes most "outdated site" cases. If it persists, switch to the desktop app or another browser, and report the problem with an [issue](https://github.com/Freeesta/mzlab/issues/new/choose) without attaching data. Support is *best effort*, with no guaranteed response time; for important lessons keep a fallback ready (desktop app installed beforehand, example files).

## Citing and reusing
The code is MIT-licensed. To cite the program use [`CITATION.cff`](../../../CITATION.cff) (GitHub's "Cite this repository" button). Third-party material has the licences listed in [`LICENZE-TERZI.md`](../../../LICENZE-TERZI.md).
