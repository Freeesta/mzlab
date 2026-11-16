# Enabling the DOI on Zenodo (once, by Federico)

`CITATION.cff` and `.zenodo.json` are already in the repository. To get a DOI for every release:

1. Go to https://zenodo.org and choose **Log in → Log in with GitHub**; authorise Zenodo's access.
2. Open https://zenodo.org/account/settings/github/ (profile menu → *GitHub*) and press **Sync now**.
3. In the list of repositories find **Freeesta/mzlab** and turn the switch **ON**. (If the repository belongs to an organisation, administrator permission is needed and the Zenodo app must be granted access to the organisation from GitHub → Settings → Applications.)
4. On GitHub create a **release** (Releases → *Draft a new release*, tag for example `v0.1.0`, title and notes, *Publish release*). Zenodo archives it by itself within a few minutes and assigns a DOI to the version and a «concept» DOI that always points to the latest.
5. On Zenodo, on the deposit page, check title, author and licence (read from `.zenodo.json`). Copy the **concept DOI**.
6. Fill in `CITATION.cff` the fields `doi`, `date-released` and `orcid` (remove the `#`) and add the same `orcid` to the creator of `.zenodo.json` (Zenodo reads ONLY `.zenodo.json`, GitHub only `CITATION.cff`: `tests/test_citazione.py` checks that title, author, ORCID, licence and keywords match), the DOI in the «How to cite» section of the README, and add the DOI badge at the top of the README (Zenodo proposes it on the deposit page).
7. Every later release creates a new version with its own DOI; the concept DOI stays the same.

Note: the content of a release cannot be changed after publication; check `CITATION.cff` before every release.
