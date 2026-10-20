<!-- inline: modi -->
<!-- script: sim-misc.js -->
<!-- title: Leggere i dati -->
# Full scan, MS2 e MRM: leggere i dati {kicker: Capitolo 12}

{.lead} Che cosa contiene ciascun tipo di file, come si estrae l'informazione utile e quali trappole tipiche della risoluzione unitaria evitare.

## Tre tipi di dato, tre forme

```html
<table>
<tr><th>Esperimento</th><th>Che cosa registra</th><th>Forma del dato</th><th>Domanda a cui risponde</th></tr>
<tr><td><b>Full scan</b> (Q1 MS)</td><td>Uno spettro completo ogni ~1 s</td><td>Matrice tempo × m/z × intensità</td><td>Che cosa c'è? Che cosa cambia nel tempo?</td></tr>
<tr><td><b>Product ion scan</b> (MS2)</td><td>Spettri dei frammenti di un precursore scelto</td><td>Matrice tempo × m/z dei frammenti</td><td>Com'è fatto questo ione?</td></tr>
<tr><td><b>MRM</b></td><td>Intensità di poche transizioni fisse</td><td>Un cromatogramma per transizione, nessuno spettro</td><td>Quanto ce n'è?</td></tr>
</table>
```

In QqQ lab il programma riconosce da solo il tipo di file (full, ms2, mrm) e propone i pannelli adatti. Nei file mzML convertiti, full scan e MS2 sono serie di spettri; l'MRM è salvato come cromatogrammi (SRM).

## I tre modi a confronto

Lo stesso testo compare nel punto interrogativo accanto a ciascuna scheda di QqQ lab. Legenda dello schema: scansiona, fisso, collisione, lascia passare.

```html
<div id="modi-all"></div>
```

## Full scan: dal cubo di dati ai cromatogrammi

### TIC, BPC, XIC

```html
<ul>
<li><b>TIC</b> (total ion current): somma di tutte le intensità di ogni spettro. Dominato dal fondo e dalle specie più abbondanti: utile per un colpo d'occhio, quasi inutile per trovare un TP minore.</li>
<li><b>BPC</b> (base peak chromatogram): per ogni spettro, l'intensità dello ione più intenso. Fa emergere meglio i picchi stretti sul fondo diffuso.</li>
<li><b>XIC</b> o EIC (extracted ion chromatogram): intensità entro una finestra di m/z attorno a un valore scelto. È lo strumento principale: un m/z = una traccia nel tempo.</li>
</ul>
```

### La finestra di estrazione

Con risoluzione unitaria il profilo di uno ione è largo ~0,7 Da a metà altezza e va da circa −0,5 a +0,5 Da dal centro. Una finestra di ±0,5 Da centrata sul valore _osservato_ raccoglie quasi tutto il segnale senza prendere i vicini. Sullo strumento del laboratorio, con l'asse spostato di circa +0,3 Da, centrare la finestra sul valore teorico taglia il picco: usate ±1 Da sul valore teorico, oppure ±0,5 sul valore osservato nello spettro dello standard.

### Spettro di un picco e sottrazione del fondo

Lo spettro "di un composto" si ottiene mediando gli spettri lungo il picco cromatografico e sottraendo quelli di una zona vicina senza picchi (o lo stesso intervallo nel bianco). Senza sottrazione, ioni del fondo (plastificanti, polisilossani, cluster della fase mobile) contaminano lo spettro e possono sembrare frammenti o addotti.

```html
<div class="box warn"><b>Errori frequenti nel full scan</b>
<ul>
<li>Considerare TP un m/z presente anche nel bianco o a t0 con intensità simile: è fondo o un'impurezza dello standard.</li>
<li>Considerare TP isotopi, addotti o frammenti in sorgente del parent: hanno lo stesso RT e la stessa forma di picco del parent (capitoli 4 e 6).</li>
<li>Confrontare le altezze di TP diversi come se fossero concentrazioni: la risposta ESI cambia da molecola a molecola.</li>
<li>Integrare XIC con picchi saturi (appiattiti in cima): l'area è sottostimata.</li>
</ul></div>
```

## Spettri a risoluzione unitaria: che cosa si può dedurre

Qui un riassunto pratico per i vostri dati; la teoria completa (masse, isotopi, regola dell'azoto, RDB, regola del 13 e il confronto con l'alta risoluzione) è nel capitolo 13.

### Pattern isotopici

Anche senza massa accurata, il **cluster isotopico** porta informazione sulla composizione:

```html
<table>
<tr><th>Elemento</th><th>Isotopo pesante</th><th>Effetto sul cluster</th></tr>
<tr><td>C</td><td><sup>13</sup>C, 1,07%</td><td>M+1 ≈ 1,1% × numero di atomi di C (stima grossolana di n<sub>C</sub>)</td></tr>
<tr><td>Cl</td><td><sup>37</sup>Cl, 24,2%</td><td>M+2 ≈ 32% per ogni Cl (rapporto ~3:1); due Cl: 100:65:10</td></tr>
<tr><td>Br</td><td><sup>81</sup>Br, 49,3%</td><td>M+2 ≈ 97% per ogni Br (rapporto ~1:1)</td></tr>
<tr><td>S</td><td><sup>34</sup>S, 4,25%</td><td>M+2 ≈ 4,4% per ogni S</td></tr>
<tr><td>N, O</td><td>piccoli</td><td>Contributi minori a M+1 (N) e M+2 (O)</td></tr>
</table>
```

Per i TP è un controllo potentissimo: se il parent contiene un cloro e il candidato mostra ancora M+2 al ~32%, il cloro è rimasto; se il pattern scompare e la massa è calata di 18 (−Cl +OH), la declorurazione idrossilativa è un'ipotesi coerente.

```html
<div class="sim" id="sim-iso"><h4>Cluster isotopico a risoluzione unitaria</h4>
<p class="note">Scrivete una formula bruta (elementi C, H, N, O, S, P, F, Cl, Br, I, Si, Na, K). Il calcolo convolve le distribuzioni isotopiche per massa nominale. L'esempio è l'atrazina, un erbicida usato come esempio svolto nel capitolo 18.</p></div>
```

### Regola dell'azoto e insaturazioni

```html
<ul>
<li><b>Regola dell'azoto</b>: una molecola neutra con massa nominale dispari contiene un numero dispari di atomi di N. Per lo ione [M+H]<sup>+</sup> vale il contrario: m/z pari ⇒ numero dispari di N. Utile per scartare formule incompatibili e per capire se una trasformazione ha tolto o aggiunto azoto.</li>
<li><b>RDBE</b> (anelli + doppi legami) = C − H/2 − X/2 + N/2 + 1 per la molecola neutra (X = alogeni). Deve essere intero e ≥ 0. Un'apertura d'anello o un'idratazione lo riducono; un'ossidazione a carbonile no.</li>
</ul>
```

## Spettri MS2

Un product ion scan registra i frammenti di un precursore isolato in Q1 con risoluzione unitaria. Alcuni punti pratici:

```html
<ul>
<li><b>Che cosa entra in cella</b>: solo il picco isotopico selezionato (di solito il monoisotopico). Se isolate invece l'M+2 di un composto clorurato, i frammenti che contengono il cloro risultano spostati di +2, quelli senza cloro no: un trucco elegante per localizzare l'alogeno.</li>
<li><b>Precursore residuo</b>: alla CE scelta parte del precursore sopravvive e compare nello spettro; serve a confermare che si sta guardando lo ione giusto.</li>
<li><b>Coeluenti isobari</b>: se allo stesso RT ci sono due molecole con la stessa massa nominale, lo spettro MS2 è la somma dei loro frammenti. Guardate gli XIC dei singoli frammenti: frammenti dello stesso precursore hanno profili cromatografici identici.</li>
<li><b>Spettri vuoti</b>: nei file MS2 molti scan sono vuoti (prima e dopo il picco). È normale; ciò che conta è lo spettro mediato sul picco.</li>
<li><b>Confronto con il parent</b>: si registrano frammento per frammento le differenze. Frammenti uguali = porzione intatta; frammenti spostati di Δ = porzione modificata (capitolo 18).</li>
</ul>
```

## MRM e quantificazione

### La retta di taratura

```html
<p class="sm">In QqQ lab si integrano i picchi MRM e si esporta la tabella delle integrazioni (aree e concentrazioni degli standard) in Excel: la retta la costruisci tu, lì.</p>
```

Si iniettano standard a concentrazione nota (ad esempio, una serie scalare dal decimo di mg/L a decine di mg/L) e si costruisce area contro concentrazione. Per una retta affidabile:

- almeno 5–6 livelli che coprano l'intervallo dei campioni (non estrapolare);
- controllare i **residui** (area misurata − area dalla retta) in funzione della concentrazione: un andamento sistematico (a "U" o a gomito) indica non linearità, tipicamente saturazione dell'ESI o del rivelatore ai livelli alti; R^2^ = 0,99 non basta a escluderla;
- su intervalli ampi, la varianza cresce con la concentrazione: una regressione pesata (1/*x* o 1/*x*^2^) evita che i punti alti dominino e che quelli bassi siano sbagliati del 50%;
- il campione che cade sopra l'ultimo standard va diluito e reiniettato.

```html
<span class="eq">LOD ≈ <span class="frac"><span>3,3 <i>s</i></span><span><i>b</i></span></span>, &nbsp;&nbsp; LOQ ≈ <span class="frac"><span>10 <i>s</i></span><span><i>b</i></span></span><span class="no">(12.1)</span></span>
```

con *b* pendenza della retta e *s* deviazione standard della risposta (del bianco o dell'intercetta). In alternativa si usa il rapporto segnale/rumore (3 e 10) su standard di bassa concentrazione.

### Conferma dell'identità

L'identità dell'analita è confermata se: RT coincide con quello dello standard (entro pochi secondi), entrambe le transizioni sono presenti, rapporto qual/quant entro la tolleranza (capitolo 10).

### Dalla quantificazione alla cinetica

Con le concentrazioni del parent si traccia *C*/*C*~0~ e ln(*C*~0~/*C*) in funzione del tempo (capitolo 2). Per i TP, senza standard, si tracciano le aree degli XIC (dal full scan) o delle transizioni MRM, eventualmente normalizzate al loro massimo: si confronta la forma della curva, non l'altezza tra TP diversi.

```html
<div class="box lab"><b>In QqQ lab</b>
<p><p class="sm"><b>Figure del picco</b> (colonne facoltative della tabella delle integrazioni): <b>S/N</b> = altezza del picco sopra la linea di base diviso la deviazione standard del segnale in una zona senza picchi scelta da te (tolta la deriva); <b>w<sub>½</sub></b> = larghezza a metà altezza; <b>N</b> = 5,54 (t<sub>R</sub> / w<sub>½</sub>)<sup>2</sup> piatti teorici; <b>coda USP</b> T = W<sub>0,05</sub> / 2f, con W<sub>0,05</sub> la larghezza e f la parte anteriore del picco a 5% dell'altezza (T = 1 per un picco simmetrico). Il programma calcola, tu interpreti.</p>
<p>La tabella delle integrazioni raccoglie le aree di ogni traccia in tutti i file e le ordina per tempo di campionamento: è il punto di partenza per i grafici cinetici e si esporta in Excel. Il menu «Correzione» offre tre modi (uno alla volta): sottrarre un <b>file bianco</b> (segue il tempo: utile quando il fondo cambia lungo la corsa), sottrarre il <b>fondo di un tratto</b> (un solo numero, la media di un tratto senza picchi: utile se il fondo è piatto) oppure la <b>linea di base automatica</b> (SNIP: toglie la base che sale sotto i picchi). Aiutano sui picchi piccoli, ma vanno usate con lo stesso criterio su tutti i tempi, altrimenti l'andamento è un artefatto.</p></div>
```

```html
<p class="note"><b>I tuoi file non lasciano il tuo computer.</b> Nella versione online i dati vengono letti dal browser e non sono inviati a nessun server; nella versione locale il programma risponde solo a questo computer.</p>
```

## Installare QqQ lab come app

```html
<ul>
<li><b>Chrome o Edge (computer):</b> nella barra degli indirizzi compare l'icona "Installa" (un monitor con una freccia): clic, poi Installa. Oppure menu a tre puntini, Installa QqQ lab.</li>
<li><b>Safari su Mac (14 o pi&ugrave; recente):</b> menu File, Aggiungi al Dock.</li>
<li><b>iPhone e iPad (Safari):</b> tasto Condividi, Aggiungi alla schermata Home.</li>
<li><b>Firefox:</b> non installa le pagine come app; funziona comunque anche senza rete dopo la prima visita.</li>
</ul>
```

Dopo la prima visita QqQ lab funziona anche senza internet, installato o no.

## Come il programma trova i bordi di un picco

Con l'integrazione automatica clicchi sul picco e il programma propone i due bordi; tu li puoi sempre correggere trascinando le barre. L'algoritmo ha quattro passi.

```html
<ol>
<li><b>Smoothing di Savitzky-Golay.</b> In ogni finestra scorrevole i punti vengono interpolati con un polinomio di secondo grado: il picco si ripulisce dal rumore senza abbassarsi e allargarsi come con una media mobile. La finestra è circa il 60% della larghezza del picco a met&agrave; altezza, quindi si adatta sia a tracce MRM con un punto ogni 0,05 s sia a un TIC con pochi punti.</li>
<li><b>Apice dalla derivata.</b> La derivata prima (la pendenza dello stesso polinomio) passa da positiva a negativa nel massimo del picco.</li>
<li><b>Bordi.</b> Si cammina dall'apice verso i due lati e ci si ferma dove il segnale scende sotto la linea di base locale pi&ugrave; il 3% dell'altezza del picco (e comunque sopra 3 volte il rumore), oppure dove la derivata cambia segno per un tratto continuo: &egrave; la valle che separa dal picco vicino.</li>
<li><b>Area.</b> Regola dei trapezi sui dati originali (non smussati), meno la linea di base retta tra i due bordi (ogni estremo &egrave; la media di 3 punti, per non dipendere da un solo punto rumoroso). L'etichetta mostra l'area con 3 decimali (2.243e+6); passando sopra compare il valore intero.</li>
</ol>
```

Limiti: due picchi sovrapposti vengono separati solo se tra i due c'è una valle; una deriva della linea di base molto forte sposta i bordi. Guarda sempre i bordi proposti prima di fidarti dell'area.

## Verifica

??? Un candidato TP ha cluster 100 : 12 : 33 : 4. Che cosa contiene probabilmente?
Un atomo di cloro (M+2 ≈ 32%) e circa 10–11 atomi di carbonio (M+1 ≈ 12%, tenendo conto del piccolo contributo dell'azoto).

```html
<details class="q"><summary>Lo ione [M+H]<sup>+</sup> del parent è a m/z dispari. Un candidato TP ha [M+H]<sup>+</sup> pari. Che cosa è cambiato?</summary>
<p>La parità del numero di atomi di azoto: il TP ha perso (o guadagnato) un numero dispari di N, oppure non è un TP del parent. Esempio: perdita di NH<sub>3</sub>, sostituzione di un gruppo amminico con OH.</p></details>
```

??? La retta MRM ha R^2^ = 0,995 ma i residui sono positivi in mezzo e negativi agli estremi. Che cosa fate?
La risposta è curva (probabilmente saturazione ai livelli alti). Restringete l'intervallo, o usate una regressione pesata/quadratica giustificata, e diluite i campioni che cadono nella zona alta.

## Per approfondire

```html
<ul class="refs">
<li><span class="tag">terminologia</span>Murray K.K. et al. (2013) Definitions of terms relating to mass spectrometry (IUPAC Recommendations 2013). <i>Pure Appl. Chem.</i> 85, 1515–1609. <a href="https://doi.org/10.1351/PAC-REC-06-04-06" target="_blank" rel="noopener">doi:10.1351/PAC-REC-06-04-06</a></li>
<li><span class="tag">libro</span>McLafferty F.W., Tureček F. <i>Interpretation of Mass Spectra</i>, 4ª ed., University Science Books, 1993.</li>
<li><span class="tag">validazione</span>Eurachem, guida <i>The Fitness for Purpose of Analytical Methods</i> (validazione, LOD, LOQ, linearità): <a href="https://www.eurachem.org" target="_blank" rel="noopener">eurachem.org</a></li>
<li><span class="tag">banche dati</span><a href="https://massbank.eu/MassBank/" target="_blank" rel="noopener">MassBank Europe</a> (spettri MS/MS liberi, molti di contaminanti ambientali); <a href="https://pubchem.ncbi.nlm.nih.gov" target="_blank" rel="noopener">PubChem</a> (formule, masse, proprietà).</li>
</ul>
```
