# Opzioni esterne: Copilot, server MCP, sandbox (D6), cloud e Worker

Indicazione di Federico: questi strumenti sono opzioni da valutare, non escluse a priori. Nessuna è autorizzata: una chat non attiva, installa, registra o usa nessuna di esse. Per usarne una si manda al Coordinatore la scheda di §6; Federico decide caso per caso.

## 1. GitHub Copilot (revisione e agente sulle PR DEV)

- **A cosa servirebbe**: revisione automatica delle PR A1–A8 (seconda lettura su schemi, `lang/`, accessibilità) o agente che apre PR piccole da un'issue (per esempio i casi d'oro nuovi). Non entra nel prodotto né nell'assistente.
- **Vantaggi**: una lettura indipendente da quella delle chat Claude; commenti direttamente sulla PR; nessun codice da mantenere.
- **Costi e rischi**: richiede un piano Copilot sull'account di Federico (la revisione e l'agente non sono nel piano gratuito); legge solo codice pubblico, ma un commento può proporre testo italiano fuori da `lang/` o nomi vietati; un'altra fonte di commenti da smistare (regole dei bot in `AGENTS.md`).
- **Alternativa locale**: `/code-review` e `/security-review` nella chat DEV, i subagenti `verificatore` e `cercatore`, `tools/pre_pr.py`.
- **Quando conviene**: se le PR DEV superano la capacità di revisione del Coordinatore o se un tipo di errore si ripete nonostante i controlli.

## 2. Server MCP di mzLab (strumenti per Claude Desktop, Cowork, Claude Code)

- **A cosa servirebbe**: esporre i passi di `mzlab-cli` (ispezione, `proposta-param/1`, `flow::check`, corsa, `tabcheck/1`) come strumenti MCP locali, così un client esterno fa da assistente senza il pannello di mzLab.
- **Vantaggi**: stesso catalogo `strumenti/1` e stessa validazione in Rust; nessuna chiave nell'app (la mette il client); utile per Federico e per le prove su dati veri senza scrivere interfaccia.
- **Costi e rischi**: un programma in più da mantenere e da firmare (processo stdio, versioni del protocollo); i risultati degli strumenti vanno al fornitore del client, quindi i dati MS escono dal computer quanto con il pannello, ma senza il nostro tetto di 10 $ né la politica L0–L2; i nomi dei file vanno nascosti dal server stesso (alias come in `05-sicurezza.md`).
- **Alternativa locale**: `mzlab-cli` dalla shell con la sequenza di `docs/agenti/per-agenti.md` (Claude Code e Cowork la usano già); il pannello A8.
- **Quando conviene**: dopo A7, se le prove mostrano che Federico usa l'assistente più da Claude Desktop o Cowork che dal pannello; il server riusa `assist` di A7 senza logica nuova.

## 3. Sandbox per JS generato (punto 4 di D6)

- **A cosa servirebbe**: eseguire codice JS scritto dal modello per grafici o calcoli che `grafico/1` e le viste nominate non coprono, in un iframe `sandbox="allow-scripts"` con CSP `default-src 'none'` e un Worker con 5 s di tempo massimo (`docs/studi/llm/README.md` D6).
- **Vantaggi**: risposte a domande insolite senza un passo Rust nuovo; tutto nel browser, nessun dato in rete in più.
- **Costi e rischi**: codice non deterministico che contraddice il principio «il modello non misura»; un numero calcolato in JS non ha parità né test; superficie di sicurezza nuova (CSP, `postMessage`); modifica di CSP e un file nuovo in `mzlab/web/llm/`; nel mondo LR può produrre la tabella o la retta che il principio 1 vieta.
- **Alternativa locale**: viste nominate `chart::views` (A5) e grafici predefiniti; codice esportato come script per l'utente (D6 punto 3); un passo Rust nuovo quando la domanda si ripete.
- **Quando conviene**: solo nel mondo HR e solo se le prove T10–T19 mostrano domande di grafico frequenti che nessuna vista copre; mai come sostituto di un calcolo con parità.

## 4. Worker e host cloud per la chiave condivisa (W1–W2)

- **A cosa servirebbe**: tenere la chiave condivisa come segreto e i contatori per persona per gli inviti (V2a, `09-worker.md`); host consigliato Cloudflare Workers Free con un Durable Object per persona (README, «Host per le prove»).
- **Vantaggi**: tetto per persona vero senza dare la chiave a nessuno; il sito funziona anche per chi non ha una chiave propria.
- **Costi e rischi**: account presso un terzo, segreto da ruotare, codice da mantenere (W1) e CSP da allargare (W2); il Worker vede le richieste in transito (contesto e domande, mai i file grezzi) e non deve registrarle (`09-worker.md` §4); limiti del piano gratuito (CPU 10 ms per richiesta); regione dei dati non garantita sul piano gratuito.
- **Alternativa locale**: chiave propria dell'utente (V1a) con il limite della sua Console; una chiave o un workspace per persona con limite di spesa (V2b); solo CLI e desktop.
- **Quando conviene**: quando più di 2–3 persone senza chiave propria devono provare l'assistente nello stesso periodo.

## 5. Corse in cloud per il test e lo sviluppo

- **A cosa servirebbe**: far girare `assist eval` e i copioni T10–T19 in una sessione cloud o con la Batch API (V3a, già scelta) invece che sul Mac; sessioni cloud per le chat DEV.
- **Vantaggi**: il Mac resta libero; corse ripetibili da zero su dati sintetici; la Batch API costa la metà.
- **Costi e rischi**: la chiave di test deve stare come segreto dell'ambiente cloud (mai nel repository né in una chat); i dati veri del repository privato non vanno nell'ambiente senza decisione di Federico; tempo di container limitato.
- **Alternativa locale**: le stesse corse sul Mac di Federico dalla CLI con il contatore locale.
- **Quando conviene**: per T1–T19 su dati sintetici (nessun dato vero); per il confronto con il chimico (§4 di `06-valutazione.md`) resta locale.

## 6. Scheda da mandare al Coordinatore prima di un uso

Una scheda per opzione e per uso, ≤ 10 righe: opzione (§1–§5); compito preciso e PR o prova che la richiede; dati che escono dal computer (sintetici, metadati, veri) e verso chi; account, piano e costo mensile; segreti e chi li crea; file del repository toccati (CSP, workflow, `Cargo.toml`, `.wasm`); come si spegne e si torna all'alternativa locale; durata. Federico risponde sì o no; senza risposta l'opzione resta spenta.
