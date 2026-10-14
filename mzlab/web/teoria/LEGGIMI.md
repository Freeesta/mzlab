# Come modificare la Teoria

Tutto il testo sta in questa cartella, un file per capitolo. Non c'è niente da compilare: modifichi, salvi, ricarichi la pagina nel browser.

## Quale file aprire

| Che cosa | File |
|---|---|
| Introduzione e mappa | `index.html` |
| Capitoli 1–22 | `01-tp.html` … `22-formulario.html` (il numero è nel nome; i vecchi nomi, come `06-quadrupolo.html`, sono rimandi automatici al nuovo capitolo) |
| Pratica (giochi ed esercizi) | `pratica.html` (pagina iniziale), `pratica-ei.html`, `pratica-orale.html`, `pratica-perdite.html`, `pratica-isotopi.html`, `pratica-formula.html`, `pratica-strumento.html`, `pratica-quadrupolo.html`, `pratica-tp.html`; codice e dati in `pratica/` |
| Domande dell'orale (testo e punti della risposta) | `pratica/domande.js` |
| Spettri EI dei capitoli 15-16 e del gioco | `pratica/ei-dati.js`, generato da `tools/genera_ei.py` (non a mano) |
| Glossario (parole sottolineate nel testo) | `21-glossario.html`, poi `python3 tools/genera_glossario.py` e `python3 tools/genera_indice_teoria.py` (ricerca) |
| Elenco dei capitoli nel menu laterale | `teoria.js`, in alto: lista `CHAPTERS` |
| Colori, caratteri, aspetto dei riquadri | `teoria.css` |
| Figure interattive (calcoli e grafici) | `sim-quad.js`, `sim-esi.js`, `sim-qqq.js`, `sim-frag.js`, `sim-misc.js`, `sim-chrom.js` (cromatografia), `sim-ei.js` (EI), `sim-hr.js` (risoluzione e formule) |

Per il testo servono solo i file `.html`. I `.js` vanno toccati solo per cambiare le figure interattive.

## Con che cosa

Un editor di testo semplice: Visual Studio Code (consigliato, colora i tag e segnala gli errori), BBEdit, oppure TextEdit **in modalità testo semplice** (Formato > Converti in testo normale; in Preferenze, "Apri file HTML come codice HTML"). Non Word.

Per vedere la modifica: apri il file con doppio clic e ricarica con Cmd+R. Nell'app, scheda Teoria, ricarica la pagina.

## Dove sta il testo

Il testo è tra i "tag" `<…>`. Si cambia il testo, si lasciano i tag. Ogni tag aperto va chiuso: `<p>` … `</p>`.

```html
<h2>Titolo di sezione</h2>          compare anche nell'indice "In questa pagina"
<h3>Sottotitolo</h3>
<p>Un paragrafo con <b>grassetto</b>, <i>corsivo</i>, H<sub>2</sub>O, 10<sup>−5</sup>.</p>
<ul><li>punto elenco</li><li>altro punto</li></ul>       (<ol> per l'elenco numerato)
<a href="06-quadrupolo.html">link a un capitolo</a>
<a href="https://doi.org/..." target="_blank" rel="noopener">link esterno</a>
```

Caratteri speciali: si possono scrivere direttamente (à è ≈ → Δ λ ±), il file è UTF-8. Gli unici da evitare nel testo sono `<` e `&`: scrivi `&lt;` e `&amp;`.

## I blocchi già pronti (copia e incolla)

```html
<div class="box key"><b>Concetto chiave</b><p>Testo.</p></div>        riquadro blu
<div class="box warn"><b>Errore frequente</b><p>Testo.</p></div>      riquadro arancione
<div class="box lab"><b>In laboratorio</b><p>Testo.</p></div>         riquadro verde
<div class="box math"><b>Derivazione</b><p>Testo.</p></div>           riquadro viola
<div class="box oral"><b>All'orale</b><p>Testo.</p></div>             che cosa si chiede all'esame
<div class="box lr">…</div> <div class="box hr">…</div>              bassa / alta risoluzione (dentro <div class="duo"> stanno affiancati)
<details class="poe"><summary>Prevedi, poi guarda</summary><p>…</p></details>   domanda prima di una figura
<div class="eispec" data-ei="2-esanone" data-keys="1"></div>        spettro EI vero (id in pratica/ei-dati.js)

<span class="eq">a = b + c<span class="no">(6.1)</span></span>         equazione centrata con numero
<span class="frac"><span>numeratore</span><span>denominatore</span></span>   frazione

<table>
<tr><th>Colonna 1</th><th class="n">Numeri</th></tr>
<tr><td>testo</td><td class="n">1,23</td></tr>
</table>                                         class="n" allinea i numeri a destra

<details class="q"><summary>Domanda di verifica?</summary>
<p>Risposta, nascosta finché lo studente non clicca.</p></details>

<li><span class="tag">articolo</span>Autore A. (anno) Titolo. <i>Rivista</i> vol, pagine. <a href="https://doi.org/..." target="_blank" rel="noopener">doi:...</a></li>
```

## Figure

- Gli schemi disegnati (`<svg>` … `</svg>`) sono dentro i capitoli. Le scritte sono nei `<text …>testo</text>`: si possono correggere; le coordinate (`x=`, `y=`) spostano le scritte.
- Le figure interattive sono i riquadri `<div class="sim" id="sim-...">`: nel capitolo si cambiano titolo (`<h4>`) e nota; il calcolo sta nel file `sim-*.js` indicato in fondo al capitolo (`<script src="sim-....js">`).
- Per aggiungere un'immagine: metti il file (`.png` o `.svg`) in questa cartella e scrivi
  `<figure><img src="nome.png" alt="descrizione" style="max-width:100%"><figcaption><b>Figura X</b> Didascalia.</figcaption></figure>`

## Aggiungere o togliere un capitolo

1. Copia un capitolo esistente, rinominalo (es. `12-nuovo.html`), cambia `kicker`, `<h1>` e testo.
2. In `teoria.js` aggiungi una riga alla lista `CHAPTERS`: `["12-nuovo.html", "12", "Titolo nel menu", "IV. Analizzare gli ioni"],` (l'ultimo campo è la parte del percorso: il menu mette un titoletto quando cambia)
3. Se vuoi, aggiungi una scheda in `index.html` (sezione "I capitoli").

Menu laterale, indice della pagina e pulsanti avanti/indietro si aggiornano da soli.

## Da non cambiare

- Le prime righe (`<!doctype …>` … `<main>`) e le ultime (`</main>` … `</html>`) di ogni capitolo.
- Gli `id="sim-..."` delle figure interattive: li usano i file `.js`.

## Se qualcosa si rompe

- Pagina vuota o figure sparite: di solito un tag non chiuso o un `"` mancante vicino all'ultima modifica.
- In Visual Studio Code i tag non chiusi sono evidenziati.
- Prima di modifiche grosse, fai un commit git (o copia il file): così puoi sempre tornare indietro.
- Controllo automatico (opzionale): `python3 -m pytest -q tests` dalla cartella del programma verifica che link, script e capitoli esistano.

## La Pratica

- Ogni gioco salva i progressi solo nel browser (`localStorage`, chiave `qqq.pratica`): nessun dato esce dal computer, nessuna classifica.
- Il motore comune è `pratica/motore.js` (livello per competenza con una regola tipo Elo, ripasso distanziato 1-3-7-21 giorni, codici sfida, conti chimici). I giochi brevi sono in `pratica/giochi.js`, il gioco EI in `pratica/gioco-ei.js`.
- Le domande dell'orale si correggono in `pratica/domande.js`: ogni domanda ha un `id` (non cambiarlo: i progressi sono salvati per id), un'area, il capitolo dove sta la risposta e i punti di una risposta completa.
- Il gioco EI e l'editor di strutture funzionano dal programma o dal sito, non da un file aperto con doppio clic (il browser blocca i moduli JavaScript da `file://`). I capitoli e gli altri giochi funzionano anche da disco.
- Spettri: solo da MassBank (licenza CC BY-NC-SA, fonte indicata sotto ogni spettro), mai dalla libreria NIST. Mai usare composti dei metodi del laboratorio.
