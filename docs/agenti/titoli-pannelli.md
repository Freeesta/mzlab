# Titoli dei pannelli

Un titolo dice solo ciò che il pannello mostra, mai un nome di file (la legenda e l'elenco dei file nominano i campioni). Le regole stanno in `mzlab/web/titoli.js` (`TITOLI.nome`, pura; `TITOLI.di(p)` legge lo stato) e sono provate da `tests/test_titoli.py` e `tests_e2e/e2e_titoli.py`. Un titolo digitato dallo studente resta com'è; i titoli predefiniti si salvano come codici (`@ms2tic`, `@ms2prec:X`, `@product:X`), mai come testo tradotto.

| Pannello | Segnale | Titolo (EN) |
|---|---|---|
| Cromatogramma | TIC / BPC | `TIC` / `BPC` (con precursore: `TIC · precursor X`) |
| Cromatogramma, scheda MS² | file con un solo precursore | `TIC · MS²` |
| Cromatogramma, scheda MS² | più precursori, nessuno scelto | `TIC · MS² (all precursors)` |
| Cromatogramma, scheda MS² | più precursori, uno scelto | `TIC · MS² · precursor X` |
| Cromatogramma PDA | tutte le λ / una / un intervallo | `PDA (all λ)` / `PDA 254 nm` / `PDA 250–260 nm` |
| XIC | una traccia / più tracce | `XIC · etichetta` / `Extracted ion (XIC)` |
| MRM, SIM | una transizione / tutte | `MRM · Q1>Q3` (`SIM · …`) / `MRM` (`SIM ions`) |
| Spettro | MS¹ / MS² senza precursore / MS² con precursore | `MS¹` / `MS²` / `MS² of X` (il tempo e l'energia restano nella barra) |
| Spettro UV | a un tempo | `UV spectrum` |
| Mappa | m/z / λ | `RT-m/z map` / `RT-λ map (PDA)` |

Spettro che segue un cromatogramma con più file sovrapposti: l'etichetta secondaria «spettro di <file>» sta nell'intestazione dello spettro (classe `.pscan`), non nel titolo del cromatogramma. Il nome del file di un'esportazione aggiunge precursore (`precX`) e λ (`PDA-254nm`).
