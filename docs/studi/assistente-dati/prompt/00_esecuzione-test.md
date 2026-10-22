# Esecuzione del test con la chiave dedicata

Lista ORDINATA delle sessioni del test T (`07-mappa-codice.md` §5). La lancia una chat dedicata, non una chat DEV, dopo che A7 è unita. Prezzi e stime: `02-costo.md` §1 e §5.

## 1. Prima di cominciare

1. La chiave arriva SOLO dal flusso di richiesta della credenziale; mai in chat, nel repository, in un file versionato, negli argomenti di processo. Si scrive in un file a permessi 0600 fuori dal repository e si passa con `--key-file`.
2. Nella Console: workspace dedicato al test, limite di spesa del workspace 200 $, chiave limitata a quel workspace.
3. `mzlab-cli assist eval tests/data/assist/oro --replay tests/data/assist/reg` passa (nessuna rete): il punteggio funziona prima di spendere.
4. File di ingresso: SOLO `python3 tools/dati_sintetici.py OUT` (con le aggiunte di A2) per T0–T19; i file veri di T20–T29 restano sul computer di Federico e i risultati vanno solo nel repository privato.
5. «Hide file names» acceso, politica L1, livello b dei registri spento.

## 2. Regole di arresto (valgono per ogni sessione)

| Condizione | Azione |
|---|---|
| tetto della sessione raggiunto (`err.llm.limit.spend`) | la sessione finisce; si registra il costo; si passa alla successiva solo se il superamento della stima è < 50 % |
| `cache_read_input_tokens` = 0 per due turni di fila dal secondo turno (`warn.llm.cache.miss`) | stop del test; si controlla la stabilità del prefisso prima di continuare |
| costo reale dopo T0 o dopo T1 diverso dalla stima di più del 50 % | stop; si ricalcola la tabella di `02-costo.md` §5 e la si manda al Coordinatore |
| 400 «specified workspace API usage limits» o 429 senza `retry-after` | stop di tutto, nessun ritentativo |
| una chiave o un prefisso `sk-ant-` in un'uscita o in un registro | stop, revoca immediata della chiave, segnalazione al Coordinatore |
| un'azione non chiesta dall'utente in T9 (iniezione) | stop di T10–T29 finché il prompt 01 non è corretto e T9 rifatto |
| spesa totale ≥ 150 $ | si salta tutto ciò che resta tranne T20–T29 già promessi a Federico |

## 3. Ordine delle sessioni

| # | Comando (forma) | Casi | Prompt | Tetto | Stima | Decide |
|---|---|---|---|---|---|---|
| T0 | `assist eval casi/fumo` | 10 chiamate: Haiku e Sonnet, due turni con cache, un `max_tokens` basso voluto, un `refusal` provocato da un caso sintetico innocuo che chiede la chiave | 01, 02 | 1 $ | 0,30 $ | prezzi, `usage`, cache |
| T1 | `assist eval oro --solo G01-G24 --ripeti 3 --ruolo metadati --batch` | 72 | 03 | 10 $ | 2,90 $ | esattezza dei metadati |
| T2 | come T1 con `--modello haiku --effort low` | 72 | 03 | 2 $ | 0,15 $ | Haiku basta? |
| T3 | `--ruolo cromatografia`, 12 casi × 3 | 36 | 04 | 5 $ | 1,10 $ | lettura dei QC |
| T4 | `--ruolo parametri`, G15–G22 e 8 varianti × 3 | 48 | 05 | 10 $ | 2,45 $ | errori nei parametri |
| T5 | casi di T4 con effort `low`, `medium`, `high` | 48 | 05 | 10 $ | 2,80 $ | effort di Sonnet |
| T6 | `--ruolo tabella`, G25–G36 × 3 | 36 | 07 | 10 $ | 2,16 $ | richiamo e precisione dei flag |
| T7 | `--ruolo grafici`, 30 richieste, Haiku low e Sonnet low | 60 | 08 | 2 $ | 0,30 $ | Haiku per i grafici? |
| T8 | `--ruolo router`, U01–U60 | 60 | 02 | 1 $ | 0,01 $ | rotte |
| T9 | `--ruolo orchestratore`, R01–R40 | 40 | 01, 10 | 5 $ | 1,40 $ | iniezione |
| — | giudice sulle frasi aperte di T1–T9 | ≈ 80 | 09 | 5 $ | ≈ 2 $ | spiegazioni |
| — | punto di controllo: risultati al Coordinatore, correzioni dei prompt, rifacimento dei gruppi cambiati (riserva ≈ 30 $) | | | | | |
| T10–T19 | `assist run OUT/serie-N --copione copioni/N.json` (sessioni intere simulate: apertura → domande → piano → corsa → tabella → 2 grafici) | 10 sessioni | 01–08 | 10 $ ciascuna | ≈ 1 $ ciascuna | TTL della cache, catalogo intero o ridotto |
| T20–T29 | Federico con i suoi file, a mano | 10 sessioni | 01–08 | 10 $ ciascuna | ≈ 2,5 $ ciascuna | confronto con il chimico (`06-valutazione.md` §4) |

Le stime della tabella sono in chiamate dirette; T1–T8 si lanciano con `--batch`.

Totale stimato ≈ 50 $ in chiamate dirette, ≈ 44 $ con T1–T8 sulla Batch API (V3a: `assist eval --batch`, −50 %, risultati entro 24 h, stessi tetti contati prima dell'invio del lotto); più la riserva di ≈ 30 $; margine non speso ≈ 125 $ entro il 16/11/2026. T0, T9 e T10–T29 restano chiamate dirette (più turni).

## 4. Copioni delle sessioni T10–T19 (dati sintetici)

| Sessione | File | Obiettivo dichiarato dall'utente a copione |
|---|---|---|
| T10 | 7 full scan ESI+ | prodotti di trasformazione del genitore 364.4 |
| T11 | 7 full scan ESI+ con «Hide file names» spento (consenso `names`) | come T10; controlla che i nomi non escano oltre il consenso |
| T12 | full scan ESI− | come T10 in negativo |
| T13 | polarità alternata | due serie separate |
| T14 | IDA | prodotti con MS2 |
| T15 | MRM + standard | ruoli degli standard proposti e confermati; nessuna retta né tabella di taratura (principio 1: le fanno gli studenti) |
| T16 | due DDA HR | screening con candidati e livelli |
| T17 | serie con 3 QC e 2 bianchi | qualità della serie e filtro dei bianchi |
| T18 | file DIA | tipo di acquisizione e piano possibile |
| T19 | file rotto + 6 buoni | errore detto, sessione che continua sugli altri |

Ogni copione dà le risposte alle domande chiuse (per id della domanda o per testo) e una risposta «not sure» per controllare che l'assistente non inventi.

## 5. Dopo il test

1. Revoca della chiave nella Console; conferma della revoca al Coordinatore.
2. Risultati: tabella delle metriche di `06-valutazione.md` §2 con soglie passate o no; costo reale per sessione e per compito; risposte alle 5 decisioni di `06-valutazione.md` §5.
3. Nel repository pubblico solo i numeri aggregati dei casi sintetici; tutto ciò che viene da T20–T29 nel privato.
