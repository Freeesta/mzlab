# Quick guide for students

mzLab lets you look at the LC-MS data of experiment 3 and search for transformation products (TPs). It shows the data: the reasoning is yours. At unit resolution **one m/z is a candidate, not an identification**.

## Opening the program
1. Open <https://freeesta.github.io/mzlab/> in a recent browser (Chrome, Edge, Firefox or Safari) on a **computer or tablet**. Nothing to install.
2. The first time, the browser downloads the calculation engine (a few tens of seconds, a network connection is needed); afterwards it is kept and later visits are faster.
3. On a **smartphone** use a computer or tablet for Data and Drawing.

## Loading data
- Drag the `.mzML` files (and the `.dam` method file, if you have it) onto the page, or use «Load more files», and check times and types in the file table.
- No files at hand: **"Try the example files"**.
- 3200 QTRAP `.wiff` files open directly: drop the `.wiff` together with its `.wiff.scan` (same name) and the site makes one mzML per sample, which you can download with «Download mzML». Other `.wiff` variants (not 3200 QTRAP) are converted with ProteoWizard MSConvert or the SCIEX converter on Windows (the guide is on the loading screen; profile mode is preferred).
- Your files **never leave your computer** (see [Privacy](privacy.md)).

## What you find
- **Data**: chromatograms (TIC, XIC, MRM), MS and MS2 spectra, RT×m/z map, peak integration with an exportable table.
- **Drawing**: a chemical editor for molecules and transformation pathways.
- The **"i"** button at the top opens the information; the **guided tour** shows the main controls.

## When something goes wrong
- Reload the page: session and drawings are restored.
- If the site looks outdated, hard-reload (Ctrl+Shift+R, or Cmd+Shift+R on Mac).
- Note the steps that lead to the problem and the browser you use, then report it with an [issue](https://github.com/Freeesta/mzlab/issues/new/choose) **without attaching laboratory data**. During lessons, ask your teacher first.
