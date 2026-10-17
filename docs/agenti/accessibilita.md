# Accessibilità di mzLab (WCAG 2.1 AA)

Questo documento riassume lo stato di conformità all'accessibilità (WCAG 2.1 livello AA, con estensioni mirate a WCAG 2.2 AA) dell'intera applicazione mzLab, i controlli automatici con axe-core, i test end-to-end con Playwright e i comandi di verifica per ciascuna area.

---

## 1. Sintesi dello stato di conformità

- **Violazioni axe-core**: **zero violazioni critiche o gravi** (`impact: critical`, `impact: serious`) in tutte le viste coperte.
- **Standard di riferimento**: WCAG 2.1 AA (e requisiti WCAG 2.2 AA selezionati: 2.5.7 Dragging Movements, 2.5.8 Target Size).
- **Screen Reader di riferimento**: Apple VoiceOver (macOS, iPadOS, iOS); secondo controllo con NVDA (Windows).
- **Tastiera e navigazione**:
  - Tutti i comandi interattivi visibili sono raggiungibili tramite sequenza sequenziale `Tab` / `Shift+Tab`.
  - Ogni elemento interattivo a fuoco mostra un indicatore visibile (outline o box-shadow ben contrastato tramite `:focus-visible`).
  - Assenza di trappole da tastiera (`keyboard traps`).
  - Tasto `Escape` chiude in modo consistente dialoghi, finestre modali, drawer, paletta comandi (`Ctrl+K`) e menu a comparsa.
  - Scorciatoie dedicate per navigazione senza trascinamento nei grafici (`+`, `-`, `[`, `]`, `0`).
- **Contrasto cromatico (WCAG 1.4.3 & 1.4.11)**:
  - Testo ordinario contro sfondo: rapporto di contrasto $\ge 4.5:1$ in modalità chiara, scura e sfondi facilitati (crema, azzurro).
  - Modalità ad alto contrasto (`Alto contrasto` / `data-a11y-bg="hc"`): rapporto $\ge 7:1$ per testo e assi cartesiani.
  - Elementi grafici e componenti dell'interfaccia: rapporto $\ge 3:1$.
  - Palette per daltonismo: adozione della tavolozza universale Okabe-Ito (2008) per serie e livelli di identificazione (Level 1 `#0072B2`, Level 2a `#009E73`, Level 2b `#56B4E9`, Level 3/3a `#E69F00`, Level 3b `#D55E00`, Level 4 `#CC79A7`, Level 5 `#999999`, Level 0 `#BBBBBB`).
  - Ridondanza dell'informazione: le serie nei grafici sono distinte anche da tratteggi (`stroke-dasharray`), e le zone di stabilità Mathieu del quadrupolo sono campite con tratteggi diagonali oltre al colore.
- **Adattamento del layout e Reflow (WCAG 1.4.10)**:
  - Supporto al reflow fino a 320 px CSS (equivalente a zoom 400% su viewport desktop standard) senza scorrimento orizzontale della pagina principale.
- **Movimento ridotto (WCAG 2.3.3)**:
  - Rispetto della media query `prefers-reduced-motion: reduce` e dell'impostazione manuale «Riduci animazioni»: animazioni e transizioni CSS disattivate o azzerate, spinner fermi, simulazioni fisiche avviate in pausa.
- **Touch target (WCAG 2.5.8)**:
  - Bersagli di tocco e controlli pannello di dimensione minima pari ad almeno 24×24 px (con espansione a 40–44 px su dispositivi touch/coarse pointer).

---

## 2. Tabella di copertura per vista

| Vista / Area | Controlli eseguiti | Standard verificati | Suite di test E2E | Stato |
| :--- | :--- | :--- | :--- | :--- |
| **Dati LR (Bassa Risoluzione)** | Controllo axe-core a vuoto e con file (Full Scan, MRM, spettri, cromatogrammi);<br>raggiungibilità da tastiera con Tab (≥90% dei comandi visibili con focus ring);<br>chiusura dialoghi con Esc; tabella testuale `t` (`TDATI`); reflow a 320 px;<br>`forced-colors`; `prefers-reduced-motion`. | WCAG 2.1 AA<br>WCAG 1.4.10<br>WCAG 2.5.7<br>WCAG 2.5.8 | `tests_e2e/e2e_a11y_dati.py`<br>`tests_e2e/e2e_axe_grafici.py` | Conforme (0 violazioni) |
| **HR (Banco Alta Risoluzione)** | Controllo axe-core sul Banco HR completo (pannelli, toolbar, barra di stato,<br>pannello informativo intestazione `#hrinfo`); dimensione impugnature pannelli<br>`.aff-grip` $\ge 24\times 24$ px. | WCAG 2.1 AA<br>WCAG 2.5.8 | `tests_e2e/e2e_axe_release.py` | Conforme (0 violazioni) |
| **Mappa 2D / 3D** | Controllo axe-core sul pannello mappa RT $\times$ m/z; zoom e pan da tastiera<br>senza drag (`+`, `-`, `[`, `]`, `0`); indicatori di fuoco visibili; contrasto assi/picchi. | WCAG 2.1 AA<br>WCAG 2.5.7 | `tests_e2e/e2e_a11y_dati.py`<br>`tests_e2e/e2e_axe_grafici.py` | Conforme (0 violazioni) |
| **Tabui / mzFinder** | Interfaccia in sola lingua inglese; etichette accessibili e ruoli ARIA per tabelle,<br>filtri e dialoghi; colori dei livelli basati su Okabe-Ito; esportazione accessibile. | WCAG 2.1 AA<br>WCAG 1.4.1 | `tests_e2e/e2e_axe_release.py`<br>`tests/test_tabui_inglese.py` | Conforme (0 violazioni) |
| **Disegno molecolare** | Controllo axe-core sull'area Disegno (escluso l'iframe isolato di Ketcher);<br>barra strumenti frammenti e formule; tastiera e contrasto dei pulsanti operativi. | WCAG 2.1 AA | `tests_e2e/e2e_axe_release.py` | Conforme (0 violazioni) |
| **Teoria e Pratica** | Struttura semantica capitoli (`lang="it"`, 1 `h1`, gerarchia `h2..h4`, `main`, `nav`, skip link);<br>pannello «Aa» (font Atkinson e OpenDyslexic su richiesta, spaziatura WCAG,<br>sfondi crema/azzurro/alto contrasto $\ge 7:1$, righello di lettura click-through);<br>descrizioni estese `<details>` per figure ed SVG; integrazione iframe `#tframe`. | WCAG 2.1 AA<br>WCAG 1.3.1<br>WCAG 1.4.3<br>WCAG 1.4.12<br>WCAG 2.4.1 | `tests_e2e/e2e_accessibilita.py`<br>`tests_e2e/e2e_axe_release.py` | Conforme (0 violazioni) |
| **Impostazioni e Palette** | Pannello `#uipset` accessibile con axe-core, apertura e chiusura via tasto Esc;<br>palette comandi `paletta` (`Ctrl+K`) apribile e chiudibile via tastiera;<br>persistenza configurazione (chiavi `mzlab.a11y` con retrocompatibilità `qqq.a11y`). | WCAG 2.1 AA<br>WCAG 2.1.2 | `tests_e2e/e2e_a11y_dati.py`<br>`tests_e2e/e2e_accessibilita.py` | Conforme (0 violazioni) |
| **Mobile e Touch** | Dimensione controlli e grip su schermi touch ($1024\times 1366$ e $400\times 800$, coarse pointer);<br>adattamento pulsante «Aa» e pannelli laterali; reflow senza scroll a 320 px CSS. | WCAG 2.1 AA<br>WCAG 1.4.10<br>WCAG 2.5.8 | `tests_e2e/e2e_axe_grafici.py`<br>`tests_e2e/e2e_a11y_dati.py`<br>`tests_e2e/e2e_accessibilita.py` | Conforme (0 violazioni) |

---

## 3. Comandi per la verifica dell'accessibilità

Tutti i test possono essere eseguiti localmente dopo aver preparato il sito con `python3 tools/prepara_sito.py`.

### 1. Test veloce Dati (fast path CI / fumo)
Esegue axe-core su 6 stati della vista Dati, controlla focus visibile da tastiera su tutti i controlli, chiusura con Esc, reflow a 320 px e modalità a contrasto forzato:
```bash
python3 tests_e2e/e2e_a11y_dati.py
```

### 2. Audit axe-core grafici e controlli pannello
Verifica che i grafici e la tabella dei dati non presentino violazioni axe, controlla la dimensione minima dei bersagli (24×24 px) e lo zoom da tastiera senza drag (`+`, `-`, `[`, `]`, `0`):
```bash
python3 tests_e2e/e2e_axe_grafici.py
```

### 3. Audit axe-core viste complementari (HR, Disegno, Pratica, Tour)
Verifica l'assenza di violazioni su Banco HR, visualizzatore molecolare Disegno, esercizi pratici e passaggi del tour guidato:
```bash
python3 tests_e2e/e2e_axe_release.py
```

### 4. Verifica completa di «Lettura facilitata» e Teoria
Controlla font ad alta leggibilità, spaziature WCAG, contrasti colore in modalità chiara/scura e alto contrasto ($\ge 7:1$), righello, navigazione da tastiera, gerarchia intestazioni e integrazione nell'app:
```bash
python3 tests_e2e/e2e_accessibilita.py
```

### 5. Verifica globale dell'applicazione (senza e2e lunghi)
Esegue la mappatura del codice, il controllo dei limiti di dimensione, verifica i18n, tipi TypeScript e import:
```bash
python3 tools/pre_pr.py --senza-e2e
```

### 6. Suite dei test unitari Python
Verifica tutte le suite unitarie del progetto:
```bash
PYTHONPATH=. pytest tests/
```
