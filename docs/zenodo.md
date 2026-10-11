# Attivare il DOI su Zenodo (una volta sola, a cura di Federico)

`CITATION.cff` e `.zenodo.json` sono già nel repository. Per ottenere un DOI a ogni release:

1. Entra su https://zenodo.org e scegli **Log in → Log in with GitHub**; autorizza l'accesso a Zenodo.
2. Apri https://zenodo.org/account/settings/github/ (menu del profilo → *GitHub*) e premi **Sync now**.
3. Nell'elenco dei repository trova **Freeesta/mzlab** e porta l'interruttore su **ON**. (Se il repository appartiene a un'organizzazione, serve il permesso di amministratore e va concesso l'accesso dell'app Zenodo all'organizzazione da GitHub → Settings → Applications.)
4. Su GitHub crea una **release** (Releases → *Draft a new release*, tag per esempio `v0.1.0`, titolo e note, *Publish release*). Zenodo la archivia da solo in pochi minuti e assegna un DOI alla versione e un DOI «concept» che punta sempre all'ultima.
5. Su Zenodo, nella pagina del deposito, controlla titolo, autore e licenza (letti da `.zenodo.json`). Copia il **concept DOI**.
6. Compila in `CITATION.cff` i campi `doi`, `date-released` e `orcid` (togli il `#`), nella sezione «Come citare» del README il DOI, e aggiungi il badge DOI in cima al README (Zenodo lo propone nella pagina del deposito).
7. Ogni release successiva crea una nuova versione con DOI proprio; il concept DOI resta lo stesso.

Nota: il contenuto di una release non si può cambiare dopo la pubblicazione; controlla `CITATION.cff` prima di ogni release.
