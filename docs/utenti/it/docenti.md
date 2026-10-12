# Guida per chi insegna

## In breve
- mzLab è un sito: gli studenti aprono <https://freeesta.github.io/mzlab/> da computer o tablet, senza installare nulla e senza account. Esiste anche un'[app desktop](desktop.md) con lo stesso aspetto.
- Il programma **non dà le risposte** (niente elenco dei TP, niente rette di taratura o tabelle della relazione): mostra i dati e gli studenti ragionano e calcolano in Excel. A risoluzione unitaria un m/z è un candidato, non un'identificazione.
- Teoria e Pratica sono solo in italiano; l'interfaccia dei Dati e del Disegno è in italiano e inglese.
- La prima apertura scarica il motore di calcolo, quindi serve la rete (in aula conviene che gli studenti aprano il sito prima della lezione).

## Prima della lezione
1. Prova il sito con i **file di esempio** («Prova con i file di esempio») e con i vostri dati.
2. Chiedi agli studenti di aprire il sito una volta da casa o dall'aula: dopo il primo caricamento il browser tiene in memoria il grosso.
3. I `.wiff` del 3200 QTRAP si aprono direttamente insieme al `.wiff.scan`; gli altri `.wiff` vanno convertiti in `.mzML` (MSConvert o convertitore SCIEX, solo Windows, formato in profilo): la guida è nella schermata di caricamento.
4. **I dati degli studenti e del laboratorio non si pubblicano**: non vanno mai in issue, forum o repository pubblici.

## Congelamento della versione durante le lezioni
Il sito si aggiorna a ogni modifica approvata. Perché in aula tutti vedano lo stesso programma, vale questa regola:

- **Nelle settimane di lezione** (le date sono nel calendario dei corsi tenuto dai manutentori) la versione è **congelata**: si pubblicano solo correzioni di errori, nessuna funzione nuova e nessun cambio dell'interfaccia.
- **Fuori dalle lezioni** si pubblica normalmente; una versione minore esce prima di ogni semestre di laboratorio.
- Chi insegna comunica ai manutentori le date dei propri corsi (con una issue di tipo «domanda») **almeno due settimane prima**, così il congelamento le copre.
- Il sito mostra sempre l'ultima versione pubblicata. Se per un esame serve sapere con precisione quale versione si è usata, annotate la data in cui l'avete usato e citatela insieme al software.

## Se un problema blocca la lezione
Ricaricare la pagina (forzando: Ctrl+Maiusc+R) risolve la maggior parte dei casi di «sito vecchio». Se persiste, passate all'app desktop o a un altro browser, e segnalate il problema con una [issue](https://github.com/Freeesta/mzlab/issues/new/choose) senza allegare dati. Il supporto è *best effort*, senza tempi garantiti; per questo, per le lezioni importanti, tenete pronto un piano alternativo (app desktop installata prima, file di esempio).

## Citare e riusare
Il codice è MIT. Per citare il programma si usa [`CITATION.cff`](../../../CITATION.cff) (il pulsante «Cite this repository» di GitHub). I testi di terze parti hanno le licenze indicate in [`LICENZE-TERZI.md`](../../../LICENZE-TERZI.md).
