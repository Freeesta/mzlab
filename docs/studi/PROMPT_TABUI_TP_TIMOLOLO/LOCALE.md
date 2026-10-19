# LOCALE — Dati, oro e letteratura sul Mac (TT-D)

Messaggio da mandare con `send_message` alla chat locale `mz-loc COO` (`session_01GZXj669sTunvu44zCFuuA7`). Sola lettura sui dati esistenti; scrive solo file NUOVI nel privato (`~/mzLab/mzlab-dati/coordinamento/consegne/`), con commit e push nel privato.

---

Studio `docs/studi/TABUI_TP_TIMOLOLO.md` (repository pubblico, §2, §3, §10). Lavoro TT-D, in sola lettura sui dati; scrivi solo i file nuovi indicati, nel privato.
1. **Oro mzmine 4.10.90** sui 13 mzML di `dati/HRMS/Timolol/` (non l'infusione): batch di riferimento di `tools/bench/oro_mzmine410.py` (stessi parametri dell'oro sintetico; scrivi nel file quali hai cambiato e perché), esporta la feature table CSV e l'MGF; salva in `coordinamento/consegne/oro_mzmine410_timololo/` (tabella, MGF, batch XML, versione).
2. **Copertura DDA**: per ogni TP di `scratchpad/timolol_truth.json` e per il genitore, in ogni file: numero di MS2 con precursore entro 10 ppm, tempi, CE del filtro, finestra di isolamento; i 10 precursori più campionati di ogni file (fondo). Tabella in `coordinamento/consegne/TABUI_TP_TIMOLOLO_DATI.md` (sezione «Copertura DDA»).
3. **Letteratura**: dai 5 articoli dello studio §10.1 (scarica i PDF aperti; per gli altri solo l'abstract) estrai TP (formula, m/z [M+H]+, frammenti, posizione proposta, livello dichiarato, DOI) in `scratchpad/timolol_lit.json`, una voce per TP, con `fonte` = DOI e `pagina`. Nessuna deduzione tua: solo ciò che è scritto.
4. **Metodo strumentale**: se Federico mette i `.raw` della serie sul Mac, installa ThermoRawFileParser (in `~/mzLab/strumenti/`, non nel repository), leggi i metadati di un `.raw` (`-m 0`) e scrivi stepped NCE, esclusione dinamica, lock mass, AGC, tempo massimo d'iniezione e risoluzione in `TABUI_TP_TIMOLOLO_DATI.md` (sezione «Metodo»). Senza `.raw`: scrivi «non disponibile».
Alla fine UN messaggio al Coordinatore COO 10 (`session_01Au8wAn7udx8RMY9ggu8RmG`): file scritti, commit del privato, cosa manca.
