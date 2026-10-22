# Mappa del codice e piano di PR

## 1. Cosa si riusa

| Pezzo | Dove | Uso nell'assistente |
|---|---|---|
| fornitori, richiesta, stream, errori HTTP | `mzlab-llm::provider` | adattatore `anthropic` con le aggiunte di A1 |
| registrazioni e LLM finto | `mzlab-llm::fake` (`llmreg/1`) | tutte le prove in CI e i casi d'oro senza rete |
| registro | `mzlab-llm::log` (`llmlog/1`, `has_secret`) | livello a, campi nuovi per ruolo e cache |
| agente come macchina a stati | `mzlab-llm::agent` | orchestratore; i subagenti sono uscite nuove della macchina |
| politica | `mzlab-llm::policy` | invariata; strumenti nuovi di classe `read`/`propose` |
| controllo dei piani | `mzlab-llm::plan` (`flow::check`, 3 tentativi) | invariato |
| contesto | `mzlab-llm::context` (`contesto/1`) | riduzioni (TIC ≤ 200, 4 cifre, alias) riusate da `ispezione/1` |
| compattazione | `mzlab-llm::compact` | invariata |
| valutazioni | `mzlab-llm::eval`, `mzlab-cli llm eval` | modello per `assist eval` |
| tetti in micro-dollari, prezzi | `mzlab-tutor::quota` | spostati in `mzlab-llm::spend` (il tutor li importa da lì) |
| controllo dei numeri e frasi | `mzlab-tutor::checks::numbers` | spostato in `mzlab-llm::text` e usato per `lettura/1` |
| trasporto `curl` con chiave su stdin | `mzlab-cli tutor eval` | stesso modo per `mzlab-cli assist` |
| stima dei parametri con prova | `mzlab-feat::params` (`features.estimate`) | segnali di `03-parametri.md` |
| metodo | `mzlab-core::metodo` (`metodo/1`) | gradiente, sorgente, esperimenti, ordine |
| ruoli | `mzlab-core::campioni` (`campioni/2`) | bianchi, QC, standard proposti |
| statistica | `table.stats`, `table.pca`, `table.kmd` | controlli di `tabcheck/1` |
| trasporti | desktop `llm/*`, sito `llm/trasporto.html` | invariati (il sito aspetta V1) |
| pannello | `flow/lazy/flow-llm.js`, `flow-llm-tools.js` | carte nuove: domande, letture, proposte |

## 2. Cosa si inventa

| Pezzo | Dove | Contratto |
|---|---|---|
| ispezione dei file | `mzlab-llm::inspect` (assemblaggio puro da risposte del motore) | `ispezione/1` |
| regole dei parametri | `mzlab-llm::params_rules` | `proposta-param/1` |
| controlli della tabella | `mzlab-feat::tabcheck` (dove sta la tabella) | `tabcheck/1` |
| specifica dei grafici | `mzlab-llm::chart` (è la D6 dello studio LLM, estesa) | `grafico/1` |
| letture dei subagenti | `mzlab-llm::reading` (validazione di prove e numeri) | `lettura/1` |
| orchestrazione dei subagenti e router | `mzlab-llm::assist` | uscite `Sub{role, request}` della macchina |
| contatore di spesa | `mzlab-llm::spend` | `assist-spesa/1` (per sessione, micro-dollari) |
| prompt come dati | `crates/mzlab-llm/data/assist/*.md` (copie dei blocchi di `prompt/`, un test confronta i byte) | SHA-256 nel registro |
| sottocomando | `mzlab-cli assist run|eval|replay` | `assist-caso/1` |
| casi d'oro | `tests/data/assist/oro/`, registrazioni `tests/data/assist/reg/` | – |

Nessun crate nuovo, nessuna dipendenza nuova, nessun cambio a `Cargo.toml`, workflow o `pages.yml`.

## 3. Correzioni al codice esistente (in A1)

- `event::Usage`: aggiungere `cache_write_5m` e `cache_write_1h` (da `usage.cache_creation`), letti dall'adattatore `anthropic`.
- `quota::Prices::list`: Sonnet 5.5 `cache_read` 0,10 (non 0,2), scritture della cache 2,50 e 4,00; Haiku 5.5 scritture 0,125 e 0,20 e la fascia sopra 100 000 token.
- `fornitori-1.json` ha i prezzi a 0 (un test vieta numeri inventati): i prezzi verificati entrano con `as_of` `2026-10-22` e la fonte nel commento del test; il test si cambia per accettare prezzi con `as_of` non vuoto. Decisione da chiedere al Coordinatore se il test lo considera un divieto assoluto.
- Adattatore `anthropic`: `cache_control` (un punto esplicito 1 h sul sistema + automatico in cima), `output_config.effort`, `thinking` adattivo, `strict: true` sugli strumenti, `output_config.format` per i subagenti, mai `temperature`/`top_p`/`top_k` (con la versione API `2026-10-21` danno 400 su ogni modello; `fornitori-1.json` manda oggi `anthropic-version: 2023-06-01`, A1 decide e prova il passaggio), messaggi `role: "system"` a metà conversazione.

## 4. Dimensione dei `.wasm` (`wasm-tetti.md`)

- Modulo `llm`: tetto `limit_llm` 286 720 B, misurato 258 709 B; margine 28 KB. `inspect`, `params_rules`, `chart`, `reading`, `spend`, `assist` sono codice di assemblaggio e aritmetica: stima +15–25 KB. Se una PR supera il tetto si ferma e dichiara la misura nuova (×1,10 a 4 KiB) come via; mai alzato in silenzio.
- `tabcheck` sta in `mzlab-feat` e arriva al sito con il modulo della tabella che già la tiene (`mzlab-feat-table-wasm`); la sua misura va nella PR A4 con lo stesso obbligo.
- Nessun byte in più per chi non accende l'assistente (profilo `corso` o interruttore spento).

## 5. Piano di PR (una alla volta, ognuna con `tools/pre_pr.py`)

| PR | Titolo | Zona | Dipende da | Rete |
|---|---|---|---|---|
| A0 | Studio e prompt (questa) | `docs/studi/assistente-dati/` | – | no |
| A1 | Adattatore Anthropic completo, spesa e prezzi verificati | `mzlab-llm` (provider, event, spend), `mzlab-tutor` (import) | A0 | no |
| A2 | `ispezione/1` e file sintetici con QC, bianchi e DIA | `mzlab-llm::inspect`, `tools/dati_sintetici.py`, schema | A1 | no |
| A3 | Regole dei parametri `proposta-param/1` | `mzlab-llm::params_rules` | A2 | no |
| A4 | Controlli della tabella `tabcheck/1` | `mzlab-feat::tabcheck`, rotta | A2 | no |
| A5 | Grafici `grafico/1` (D6 dello studio LLM, estesa) | `mzlab-llm::chart`, pannello | A1 | no |
| A6 | Subagenti, router, `lettura/1`, prompt come dati | `mzlab-llm::{assist, reading}`, `data/assist/` | A3, A4, A5 | no |
| A7 | `mzlab-cli assist run|eval|replay` e casi d'oro | `mzlab-cli`, `tests/data/assist/` | A6 | solo a mano |
| T | Test con la chiave (`prompt/00_esecuzione-test.md`) | nessun codice; risultati nel privato | A7 | sì, chiave dedicata |
| A8 | Pannello dell'assistente nel desktop (carte domande, letture, proposte) | `flow/lazy/flow-llm*.js`, `lang/` | T | no |
| A9 | Sito | `llm/trasporto.html` | V1 | – |

Prompt per le chat DEV: `08-prompt-dev.md`. A1–A7 sono la stessa chat DEV (zona `mzlab-llm`), A8 una seconda chat (zona pagina). Il test T lo lancia una chat con la chiave, mai una chat DEV.

## 6. Conflitti caldi

- `crates/mzlab-llm/src/lib.rs` (moduli), `mzlab-cli/src/main.rs` (sottocomandi), `docs/schemi/strumenti-1.json` (strumenti nuovi: si rigenera con `MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-cli --lib llm_tools` a fine fusione, mai a mano).
- Lo studio LLM (`docs/studi/llm/README.md` §13 D6) e A5 descrivono lo stesso modulo `chart`: A5 lo sostituisce e la riga di D6 si aggiorna nella stessa PR.
