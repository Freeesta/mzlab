<!-- no-appname -->
<!-- title: Alta risoluzione e DDA -->
# Alta risoluzione e acquisizione dipendente dai dati {kicker: Capitolo 19}

{.lead} Il triplo quadrupolo del laboratorio lavora a risoluzione unitaria: un *m/z* è un numero intero con qualche decimale che dipende dalla taratura. Uno spettrometro Orbitrap o Q-TOF distingue masse che differiscono di pochi millesimi, e di solito sceglie da solo gli ioni da frammentare. Questo capitolo spiega le parole che servono per leggere quei dati; il programma li riconosce da solo e cambia di conseguenza decimali e tolleranze.

## Potere risolutivo, accuratezza e ppm

```html
<p>Il <b>potere risolutivo</b> <i>R</i> = <i>m</i> / &Delta;<i>m</i> dice quanto sono strette le righe: &Delta;<i>m</i> è la larghezza del picco a metà altezza. A <i>m/z</i> 300 un <i>R</i> di 60&thinsp;000 corrisponde a righe larghe 5 m<i>Da</i> (0.005 <i>m/z</i>); a risoluzione unitaria la larghezza è circa 0.7. L'<b>accuratezza di massa</b> è un'altra cosa: quanto il valore misurato si discosta da quello vero. Si esprime in <b>parti per milione</b>:</p>
```

```html
<div class="box"><b>Errore di massa</b><p>errore (ppm) = (<i>m/z</i> osservata &minus; <i>m/z</i> calcolata) / <i>m/z</i> calcolata &times; 10<sup>6</sup></p></div>
```

```html
<p>Esempio (atrazina, C<sub>8</sub>H<sub>14</sub>ClN<sub>5</sub>): [M+H]<sup>+</sup> calcolato 216.1010; se lo strumento misura 216.1013, l'errore è (216.1013 &minus; 216.1010) / 216.1010 &times; 10<sup>6</sup> = +1.4 ppm. Per questo la tolleranza di un XIC o di un confronto non è in Da ma in ppm: 5 ppm sono 1.1 m<i>Da</i> a <i>m/z</i> 216 e 2.5 m<i>Da</i> a <i>m/z</i> 500. Il valore predefinito del programma (5 ppm) è lo stesso di Thermo FreeStyle e si cambia nell'ingranaggio, riga &laquo;Masse&raquo;.</p>
```

Un errore sistematico (tutti gli ioni spostati dello stesso valore, per esempio +3 ppm) indica una taratura da rivedere; un errore che cambia da ione a ione e da scansione a scansione è rumore. La calcolatrice *m/z* permette di scrivere la massa osservata e mostra l'errore in ppm di ogni addotto: il confronto lo interpreti tu.

## Difetto di massa e masse quasi uguali

Le masse esatte non sono numeri interi: il **difetto di massa** (la parte decimale) dipende da quali atomi ci sono. A risoluzione unitaria CO e C~2~H~4~ pesano entrambi 28; a risoluzione alta no:

```html
<table class="t"><thead><tr><th>Perdita</th><th>Massa esatta</th><th>Differenza</th></tr></thead><tbody>
<tr><td>CO</td><td>27.9949</td><td rowspan="2">0.0364</td></tr><tr><td>C<sub>2</sub>H<sub>4</sub></td><td>28.0313</td></tr>
<tr><td>CO<sub>2</sub></td><td>43.9898</td><td rowspan="2">0.0728</td></tr><tr><td>C<sub>3</sub>H<sub>8</sub></td><td>44.0626</td></tr></tbody></table>
```

Per vedere due picchi separati servono righe più strette della loro distanza: a *m/z* 400 la differenza di 0.0364 richiede *R* > 11 000, che Orbitrap e Q-TOF hanno, un quadrupolo no. Due ioni **isobari** (stessa massa nominale, formule diverse) che a risoluzione unitaria sono un unico picco diventano due: puoi estrarre l'XIC dell'uno senza l'altro. Resta il limite opposto: stesse masse esatte (isomeri) non si separano con nessuna risoluzione.

## DDA: lo strumento sceglie cosa frammentare

Nell'acquisizione **dipendente dai dati** (DDA, in Analyst IDA) lo strumento esegue un ciclo: una scansione di tutti gli ioni (**Full Scan**, MS1) e poi, per i *N* ioni più intensi, una scansione di frammentazione ciascuno (**MS2**). Le regole sono impostate nel metodo:

```html
<ul>
<li><b>top-N</b>: quanti ioni frammentare in ogni ciclo (3, 10, 20&hellip;);</li>
<li><b>soglia di intensità</b>: sotto un valore l'ione non viene scelto;</li>
<li><b>esclusione dinamica</b>: uno ione già frammentato non viene scelto di nuovo per un certo tempo (per esempio 0.3 min), così il ciclo passa ai successivi invece di ripetersi sullo stesso picco;</li>
<li>di norma <b>una MS2 per picco cromatografico</b> (qualche MS2 per un picco largo): non c'è garanzia che lo strumento abbia frammentato <i>il</i> momento migliore, né che abbia frammentato <i>il tuo</i> ione.</li></ul>
```

Per questo ogni MS2 ricorda da quale Full Scan è partita (**scansione madre**) e quale **finestra di isolamento** è stata usata. Nel programma la Full Scan e la MS2 stanno una accanto all'altra sotto il cromatogramma: una bandierina ▼ sopra un picco della Full Scan dice che da lì è partita una MS2 (un clic la mostra); un triangolo vuoto ▽ dice che quello ione è stato frammentato in un'altra scansione vicina nel tempo; con «Segui questo ione nelle MS2» vedi sul suo XIC dove lo strumento ha preso le MS2 e le scorri con le frecce. Se uno ione non è mai stato scelto, il programma lo dice: «Nessuna MS2…: il DDA non l'ha scelto».

## Finestra di isolamento e spettri misti

Prima della frammentazione un quadrupolo lascia passare una finestra di *m/z* attorno al precursore (±0.5, ±0.75, ±1.5…). Tutto ciò che sta dentro la finestra viene frammentato insieme: se due ioni sono più vicini della larghezza della finestra, la MS2 è uno **spettro misto** con i frammenti di entrambi. In pratica: uno ione debole accanto a uno forte non avrà una MS2 sua (lo strumento sceglie il forte), ma i suoi frammenti possono comparire nella MS2 del forte. Per questo conviene guardare nella Full Scan la banda di isolamento (rettangolo colorato) prima di attribuire i frammenti a un precursore.

## L'energia di collisione: eV oppure NCE

Nel 3200 QTRAP l'energia di collisione è in **eV**. Negli strumenti Thermo (Orbitrap) si usa l'energia di collisione **normalizzata** (**NCE**, in %): un numero relativo alla massa e alla carica del precursore, senza unità. NCE 30 non è 30 eV e due strumenti con lo stesso NCE non sono confrontabili punto per punto. Il programma scrive «NCE 30» per i file Thermo e «CE 25 eV» per gli altri. L'**attivazione** può essere HCD (collisione in una cella, tipica degli Orbitrap) o CID (in trappola): gli spettri sono diversi, come nel capitolo 14.

## Preparare i file

Il programma legge file mzML con **centroidi**, cioè un punto per picco. Da un file Thermo .raw si ottengono con ProteoWizard MSConvert (peak picking del costruttore, compressione zlib):

```html
<pre>msconvert file.raw --mzML --zlib --filter "peakPicking vendor msLevel=1-"</pre>
```

Per alleggerire un file molto grande si possono tenere solo i minuti che servono e gli ioni più intensi:

```html
<pre>msconvert file.raw --mzML --zlib --filter "peakPicking vendor msLevel=1-" --filter "scanTime [600,1500]" --filter "threshold count 300 most-intense"</pre>
```

(«scanTime» è in secondi.) Un file in profilo ad alta risoluzione pesa molto di più e il browser fatica a tenerlo in memoria: il programma lo segnala.

## Dalla massa esatta alla formula (strumenti dell'alta risoluzione)

Con i file ad alta risoluzione il programma offre alcuni strumenti in più. Nessuno dà una risposta sicura: danno **candidati**, e sta a te giudicarli.

```html
<ul>
<li><b>Formule compatibili.</b> Clic destro su un picco di uno spettro: il programma elenca le formule che hanno quella massa esatta entro la tolleranza (ppm o mDa), con gli elementi che consenti. Per ognuna mostra l'errore, il numero di anelli e doppi legami (RDB), le &laquo;sette regole d'oro&raquo; e, se i picchi M+1 e M+2 ci sono, se gli isotopi sono compatibili. Più elementi consenti, più candidati compaiono: restringi l'elenco a quelli che puoi davvero avere.</li>
<li><b>Profilo isotopico alla risoluzione dello strumento.</b> Il profilo teorico di una formula tiene conto del potere risolutivo della scansione: a <i>R</i> di qualche decina di migliaia i picchi di M+2 dovuti a <sup>34</sup>S, <sup>13</sup>C<sub>2</sub> e <sup>18</sup>O sono troppo vicini e si fondono in un picco solo; a risoluzione più alta si separano. Dal menu del profilo puoi cambiare <i>R</i>.</li>
<li><b>Albero MSn.</b> Per un file di infusione con più stadi di frammentazione (MS3, MS4&hellip;), il menu del cromatogramma apre l'albero dei percorsi: per ogni nodo lo spettro, il precursore esatto e la formula, che deve essere una parte di quella del nodo padre. I nodi quasi vuoti e i picchi che non c'entrano (contaminazioni) sono segnalati.</li>
<li><b>Cerca nelle librerie.</b> Per una MS2 il programma confronta lo spettro con le librerie che hai caricato e dà un giudizio prudente (corrispondenza forte, possibile, debole); la ricerca per <b>analoghi</b> trova composti simili anche con la massa diversa, spiegando la differenza (per esempio +O per un'ossidrilazione).</li>
<li><b>Rilevamento dei picchi.</b> Dal menu dell'integrazione puoi cambiare i parametri dell'integrazione automatica (soglia di rumore, bordi, risoluzione tra due picchi vicini). I valori di partenza sono quelli di sempre.</li>
</ul>
```
