# Contribuire a mzLab / Contributing

mzLab è un programma **didattico** per il laboratorio di analisi degli inquinanti (UniTO). Il progetto è mantenuto *best effort*: le segnalazioni e le proposte sono benvenute, ma i tempi di risposta non sono garantiti. / mzLab is a **teaching** program, maintained on a best-effort basis: reports and proposals are welcome, with no guaranteed response time.

## Segnalare un problema / Reporting
- Errori e domande: [nuova issue](https://github.com/Freeesta/mzlab/issues/new/choose). **Mai allegare dati di laboratorio, file degli studenti o dati personali**: il repository è pubblico. / Never attach laboratory data, student files or personal data: the repository is public.
- Vulnerabilità: non in una issue pubblica, vedi [SECURITY.md](SECURITY.md).
- Guide per chi usa il programma: [docs/utenti/](docs/utenti/README.md).

## Proporre una modifica / Proposing a change
1. Apri prima una issue per descrivere la modifica, soprattutto se cambia il comportamento o l'interfaccia.
2. Lavora su un ramo `claude/<argomento>` o `<tuo-nome>/<argomento>` partendo da `main`; una pull request per argomento. L'unione avviene con *merge commit* (niente rebase né push forzato) dopo la CI verde.
3. Le regole del progetto sono in [AGENTS.md](AGENTS.md) e in [docs/agenti/](docs/agenti/): principi didattici (il programma non dà le risposte), leggerezza (nessun bundler), interfaccia in italiano e inglese tramite i cataloghi in `mzlab/web/lang/`, accessibilità (WCAG 2.2 AA, ogni grafico con la sua tabella dei dati), architettura e test.
4. Prima della PR: `python3 tools/verifica.py --cambiati` (vedi `docs/agenti/test.md`).
5. **Teoria e Pratica** (`mzlab/web/teoria/`) si modificano solo su indicazione esplicita del responsabile del corso.

## Dati e contenuti / Data and content
Nel repository non entrano dati del laboratorio, file degli studenti né cognomi di persone del corso: stanno in un repository privato. Gli esempi pubblici sono anonimi. / No laboratory data, student files or surnames of course people go in the repository.

## Licenza / Licence
Con la pull request accetti che il tuo contributo sia rilasciato con la licenza MIT del progetto ([LICENSE](LICENSE)).
