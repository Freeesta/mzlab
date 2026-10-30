# Build prompts: EU and US regulatory lists

One file per package. The plan, the sources and the decisions are in `docs/studi/LISTE_NORMATIVE.md`. Every prompt is self-contained: copy the part after "---".

Order: A can start now. B needs A merged and Federico's decision D1. C needs B's format (it can start on A's fixture) and must not run in parallel with a chat that owns `screen.suspects`. D needs A and B.

- [A](A_builder.md): builder `mzlab-cli normative build` (RG-04 + list part of RG-06): official sources → per-list A1+ CSV + provenance
- [B](B_pacchetto.md): first real build committed, `.mzsus` `listings` extension, package in the site (CI, no network) and in the app
- [C](C_screening.md): `screen.suspects` with the lists, the `annot.regulatory` executor, the Library panel, user import of ECHA/OEHHA files, report wording
- [D](D_aggiornamento.md): `normative check`, `normative diff`, the monthly update routine

If the COO changes, replace the session id in every file (`grep -l session_01NawSmneBkv8sosEHpoC8k4 docs/studi/prompt_liste_normative/*`).
