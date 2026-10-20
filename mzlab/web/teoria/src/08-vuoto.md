<!-- script: sim-misc.js -->
# Dalla sorgente al vuoto {kicker: Capitolo 8}

{.lead} Gli ioni nascono a pressione atmosferica, ma un quadrupolo funziona solo se lo ione lo attraversa senza urtare nulla. In pochi centimetri la pressione scende di otto ordini di grandezza.

## Perché serve il vuoto

La separazione in massa di un quadrupolo si basa sulla traiettoria di uno ione in un campo elettrico (capitolo 9). Ogni urto con una molecola di gas cambia velocità e direzione dello ione: se gli urti sono frequenti, la traiettoria non dipende più da m/z e la separazione si perde. La grandezza che decide è il **libero cammino medio**, la distanza media percorsa tra due urti:

```html
<span class="eq"><i>λ</i> = <span class="frac"><span><i>k</i><sub>B</sub><i>T</i></span><span>√2 π <i>d</i><sup>2</sup> <i>p</i></span></span><span class="no">(8.1)</span></span>
```

dove *d* è il diametro di collisione (per N~2~ circa 0,37 nm; per uno ione organico il diametro, e quindi la sezione d'urto, è maggiore: i valori calcolati sono un limite superiore). Un quadrupolo è lungo circa 20 cm: per attraversarlo senza urti serve *λ* di metri, cioè pressioni intorno a 10^−5^ Torr.

```html
<div class="sim" id="sim-mfp"><h4>Libero cammino medio e pressione</h4>
<p class="note">Equazione 5.1 per N<sub>2</sub> a 25 °C. I punti arancioni indicano gli stadi di pressione tipici di uno strumento API.</p></div>
```

## Gli stadi di pompaggio

Il passaggio da 760 Torr all'alto vuoto avviene con **vuoto differenziale**: piccoli orifizi separano camere pompate in modo indipendente (pompa primaria rotativa o scroll sugli stadi a pressione alta, turbomolecolare a più stadi sull'analizzatore). Ogni orifizio lascia passare un flusso di gas limitato, ma gli ioni, guidati da campi elettrici, passano con efficienza molto maggiore delle molecole neutre.

```html
<table>
<tr><th>Regione</th><th class="n">Pressione tipica</th><th class="n">λ (N<sub>2</sub>)</th><th>Che cosa succede agli ioni</th></tr>
<tr><td>Sorgente</td><td class="n">760 Torr</td><td class="n">~70 nm</td><td>Moto dominato dal gas e dal campo; desolvatazione</td></tr>
<tr><td>Dopo orifizio/skimmer</td><td class="n">~1 Torr</td><td class="n">~50 µm</td><td>Espansione supersonica; il DP accelera gli ioni contro il gas: declustering (o frammentazione in sorgente)</td></tr>
<tr><td>Q0, guida di ioni</td><td class="n">mTorr</td><td class="n">mm–cm</td><td>Centinaia di urti a bassa energia: gli ioni si "raffreddano" e si concentrano sull'asse (focalizzazione collisionale)</td></tr>
<tr><td>Q1, Q3</td><td class="n">~10<sup>−5</sup> Torr</td><td class="n">metri</td><td>Traiettorie senza urti: filtro di massa</td></tr>
<tr><td>q2, cella di collisione</td><td class="n">mTorr di N<sub>2</sub></td><td class="n">mm</td><td>Urti voluti, ad energia controllata: CID (capitolo 10)</td></tr>
</table>
```

## Il gas di tenda e il declustering

Nella geometria SCIEX una corrente di N~2~ puro (curtain gas, CUR) esce dallo spazio tra piastra di tenda e orifizio verso la sorgente. Gli ioni la attraversano perché sono spinti dal campo elettrico; goccioline, solvente e neutri vengono respinti. Si riduce così l'ingresso di acqua nel vuoto, la formazione di cluster ione–solvente e la contaminazione dell'orifizio.

```html
<p>Il <b>declustering potential</b> (DP) è la differenza di potenziale tra orifizio e la regione successiva. Accelera gli ioni nella zona a ~1 Torr, dove gli urti con il gas strappano le molecole di solvente ancora legate ([M+H(H<sub>2</sub>O)<sub><i>n</i></sub>]<sup>+</sup> → [M+H]<sup>+</sup>). È un compromesso:</p>
```

- DP troppo basso: cluster e addotti, segnale distribuito su più m/z, rumore chimico;
- DP troppo alto: lo ione molecolare si frammenta prima di Q1 (**CID in sorgente**). Nel full scan compaiono "frammenti" allo stesso tempo di ritenzione del parent.

```html
<div class="box lab"><b>Nei metodi del laboratorio</b>
<p>Il DP fa parte dei parametri "di composto" (si ottimizza per ogni analita, come la CE); nel metodo full scan è un valore unico per tutti gli ioni, quindi non ottimale per ciascuno. Un TP fragile può mostrare in full scan frammenti in sorgente: tenetelo presente quando un "nuovo" m/z ha esattamente lo stesso RT e la stessa forma di picco di un altro.</p></div>
```

## Ottiche ioniche a radiofrequenza

Tra gli stadi gli ioni sono guidati da multipoli alimentati solo in radiofrequenza (quadrupoli, esapoli, ottapoli "RF-only"). Senza componente continua, un multipolo non separa le masse ma crea un pozzo di potenziale efficace che confina sull'asse tutti gli ioni sopra una certa massa (capitolo 9, regione *a* = 0). In presenza di gas, gli urti dissipano l'energia degli ioni, che si concentrano sull'asse del pozzo: la trasmissione verso Q1 aumenta molto. Lo stesso principio, con energia più alta, tiene insieme i frammenti nella cella di collisione.

## Verifica

??? Perché un quadrupolo non funzionerebbe a 1 mTorr?
A 1 mTorr il libero cammino medio è di qualche centimetro: in 20 cm lo ione subirebbe molti urti e la sua traiettoria non dipenderebbe più solo da m/z e dai campi applicati; risoluzione e trasmissione crollerebbero.

??? Aumentando il CUR il segnale cala leggermente ma il fondo cala molto. Perché?
Più gas di tenda respinge meglio neutri, cluster e goccioline (fonte di rumore chimico), ma rallenta anche parte degli ioni. Il rapporto segnale/rumore spesso migliora.

## Per approfondire

```html
<ul class="refs">
<li><span class="tag">libro</span>Gross J.H. (2017) <i>Mass Spectrometry: A Textbook</i>, 3ª ed., Springer, capitoli sull'interfaccia API e sul vuoto. <a href="https://doi.org/10.1007/978-3-319-54398-7" target="_blank" rel="noopener">doi:10.1007/978-3-319-54398-7</a></li>
<li><span class="tag">libro</span>de Hoffmann E., Stroobant V. (2007) <i>Mass Spectrometry: Principles and Applications</i>, 3ª ed., Wiley.</li>
<li><span class="tag">web</span><a href="https://en.wikipedia.org/wiki/Mean_free_path" target="_blank" rel="noopener">Mean free path</a> (Wikipedia).</li>
</ul>
```
