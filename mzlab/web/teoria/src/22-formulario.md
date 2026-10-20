<!-- inline: formulario-quiz -->
<!-- full-title: Formulario · Teoria mzLab -->
# Formulario essenziale {kicker: Capitolo 22}

{.lead} Le relazioni fondamentali di cromatografia, sorgenti, vuoto, analizzatori ed elaborazione degli spettri. Per ciascuna formula: significato fisico, parametri critici, ordini di grandezza tipici e capitolo di derivazione.

```html
<div class="box key"><b>Come usarlo</b>
<p>Non serve memorizzare le formule come stringhe astratte di simboli, ma comprenderne a fondo il significato operativo. Per ogni relazione è utile chiedersi: <b>da che cosa dipende</b> (cosa accade al variare di ciascun parametro?), <b>da dove deriva</b> (quale principio fisico governa il fenomeno) e <b>quali ordini di grandezza</b> caratterizzano la strumentazione reale. Con il pulsante sottostante le formule possono essere nascoste per testare la propria preparazione.</p>
<p class="noprint"><button id="hide" type="button">Nascondi le formule (mettiti alla prova)</button></p></div>
```

{.note} Simboli comuni: *z* numero di cariche, *e* = 1,602·10^−19^ C carica elementare, *m* massa dello ione in kg (1 u = 1,6605·10^−27^ kg), *k*~B~ = 1,381·10^−23^ J/K, 1 eV = 96,49 kJ/mol. Nelle formule degli analizzatori *m*/*z* compare come *m*/(*ze*): con *m* in u e *z* intero si ottiene il valore che leggete sullo spettro.

## 1. Cromatografia (capitoli 3, 4, 5)

### Ritenzione e selettività

```html
<span class="eq"><i>k</i> = <span class="frac"><span><i>t</i><sub>R</sub> − <i>t</i><sub>0</sub></span><span><i>t</i><sub>0</sub></span></span> = <span class="frac"><span><i>K</i></span><span>β</span></span>, &nbsp;&nbsp; <i>t</i><sub>R</sub> = <i>t</i><sub>0</sub>(1 + <i>k</i>), &nbsp;&nbsp; α = <span class="frac"><span><i>k</i><sub>2</sub></span><span><i>k</i><sub>1</sub></span></span><span class="no">(3.1–3.2)</span></span>
```

```html
<p>Fattore di ritenzione <i>k</i>: quante volte più tempo l'analita passa nella fase stazionaria che nella mobile. <i>K</i> è la costante di ripartizione, β il rapporto di fase (volume di fase mobile / volume di fase stazionaria). Selettività α: rapporto dei <i>k</i> di due composti vicini, dipende solo dalla chimica (fasi, temperatura, solvente), non dalla colonna lunga o corta. Buoni valori: 1 &lt; <i>k</i> &lt; 10.</p>
```

```html
<span class="eq">β = <span class="frac"><span><i>r</i><sub>c</sub></span><span>2 <i>d</i><sub>f</sub></span></span> &nbsp;&nbsp; (colonna capillare GC)</span>
```

Raggio interno della colonna su due volte lo spessore del film. Colonna 0,25 mm × 0,25 µm: β = 125 / 0,5 = 250. Film più spesso → β più piccolo → *k* più grande (più ritenzione, utile per i volatili).

### Efficienza: piatti teorici e allargamento

```html
<span class="eq"><i>N</i> = <span class="frac"><span><i>t</i><sub>R</sub><sup>2</sup></span><span>σ<sup>2</sup></span></span> = 16 <span class="frac"><span><i>t</i><sub>R</sub><sup>2</sup></span><span><i>w</i><sub>b</sub><sup>2</sup></span></span> = 5,54 <span class="frac"><span><i>t</i><sub>R</sub><sup>2</sup></span><span><i>w</i><sub>1/2</sub><sup>2</sup></span></span>, &nbsp;&nbsp; <i>H</i> = <span class="frac"><span><i>L</i></span><span><i>N</i></span></span><span class="no">(3.3)</span></span>
```

Picco gaussiano: larghezza di base *w*~b~ = 4σ, a metà altezza *w*~1/2~ = 2,355σ (5,54 = 2,355^2^). *H* (altezza equivalente a un piatto) è la grandezza che si confronta fra colonne: per una colonna impaccata HPLC buona *H* ≈ 2–2,5 *d*~p~ (particelle da 3 µm → *H* ≈ 7 µm, cioè ≈ 14 000 piatti in 10 cm).

```html
<span class="eq"><i>H</i> = <i>A</i> + <span class="frac"><span><i>B</i></span><span><i>u</i></span></span> + <i>C</i> · <i>u</i>, &nbsp;&nbsp; <i>u</i><sub>opt</sub> = √(<i>B</i>/<i>C</i>), &nbsp;&nbsp; <i>H</i><sub>min</sub> = <i>A</i> + 2√(<i>BC</i>)<span class="no">(3.4)</span></span>
```

Van Deemter: *A* cammini multipli (solo colonne impaccate; nella capillare GC *A* = 0: equazione di Golay), *B*/*u* diffusione longitudinale (∝ coefficiente di diffusione *D*~m~: grande nei gas), *C*·*u* trasferimento di massa (∝ *d*~p~^2^/*D*~m~). Per questo in GC H~2~ ed He hanno ottimi più veloci e più piatti di N~2~, e in HPLC le particelle piccole spostano il minimo a velocità più alte. Velocità lineare media: *ū* = *L*/*t*~0~.

### Risoluzione

```html
<span class="eq"><i>R</i><sub>s</sub> = <span class="frac"><span>2 (<i>t</i><sub>R,2</sub> − <i>t</i><sub>R,1</sub>)</span><span><i>w</i><sub>b,1</sub> + <i>w</i><sub>b,2</sub></span></span> = 1,18 <span class="frac"><span><i>t</i><sub>R,2</sub> − <i>t</i><sub>R,1</sub></span><span><i>w</i><sub>1/2,1</sub> + <i>w</i><sub>1/2,2</sub></span></span><span class="no">(3.5)</span></span>
```

```html
<span class="eq"><i>R</i><sub>s</sub> = <span class="frac"><span>√<i>N</i></span><span>4</span></span> · <span class="frac"><span>α − 1</span><span>α</span></span> · <span class="frac"><span><i>k</i><sub>2</sub></span><span>1 + <i>k</i><sub>2</sub></span></span><span class="no">(3.6)</span></span>
```

Equazione di Purnell: tre leve indipendenti, efficienza, selettività, ritenzione. *R*~s~ = 1,5 è la separazione «alla linea di base». Esempio: *N* = 10 000, α = 1,05, *k* = 5 → *R*~s~ = 25 × 0,048 × 0,83 ≈ 1,0. Raddoppiare *N* (colonna lunga il doppio, tempo doppio) dà solo √2 = 1,41 volte la risoluzione; portare α da 1,05 a 1,10 la quasi raddoppia. Da qui i piatti necessari: *N*~req~ = 16 *R*~s~^2^ (α/(α−1))^2^ ((1+*k*)/*k*)^2^.

```html
<span class="eq"><i>n</i><sub>c</sub> ≈ 1 + <span class="frac"><span><i>t</i><sub>g</sub></span><span><i>w</i></span></span> &nbsp;&nbsp; (capacità di picchi in gradiente)</span>
```

Quanti picchi ci stanno, uno accanto all'altro, nel tempo del gradiente *t*~g~ con larghezza di base media *w*. Gradiente di 20 min, picchi larghi 12 s → circa 100.

### Indici di ritenzione (GC)

```html
<span class="eq"><i>I</i> = 100 [<i>n</i> + <span class="frac"><span>log <i>t</i>'<sub><i>x</i></sub> − log <i>t</i>'<sub><i>n</i></sub></span><span>log <i>t</i>'<sub><i>n</i>+1</sub> − log <i>t</i>'<sub><i>n</i></sub></span></span>] &nbsp; (isoterma), &nbsp;&nbsp; <i>I</i> = 100 [<i>n</i> + <span class="frac"><span><i>t</i><sub><i>x</i></sub> − <i>t</i><sub><i>n</i></sub></span><span><i>t</i><sub><i>n</i>+1</sub> − <i>t</i><sub><i>n</i></sub></span></span>] &nbsp; (programmata)<span class="no">(5.1–5.2)</span></span>
```

```html
<p>Il tempo di ritenzione riportato sulla scala degli <i>n</i>-alcani (C<sub><i>n</i></sub> ha indice 100<i>n</i>). Indipendente da lunghezza della colonna e flusso: insieme allo spettro EI è il secondo criterio di identificazione in GC-MS. <i>t</i>' = <i>t</i><sub>R</sub> − <i>t</i><sub>0</sub> tempo ridotto.</p>
```

### HPLC: fase inversa e pressione

```html
<span class="eq">log <i>k</i> ≈ log <i>k</i><sub>w</sub> − <i>S</i> φ<span class="no">(4.1)</span></span>
```

Modello del solvente forte (LSS): in fase inversa log *k* cala linearmente con la frazione di organico φ. *S* ≈ 3–5 per molecole piccole: +10% di metanolo divide *k* per circa 2–3. È la base del gradiente.

```html
<span class="eq">Δ<i>P</i> = <span class="frac"><span>φ<sub>R</sub> η <i>L</i> <i>u</i></span><span><i>d</i><sub>p</sub><sup>2</sup></span></span> &nbsp;&nbsp; (legge di Darcy, φ<sub>R</sub> ≈ 500–1000)</span>
```

La pressione cresce con viscosità η, lunghezza e velocità, e con l'inverso del **quadrato** del diametro delle particelle: passando da 5 a 1,7 µm la pressione aumenta di quasi 9 volte (UHPLC). Esempio: acqua (η = 1 mPa·s), 10 cm, *u* = 2 mm/s, 3 µm, φ~R~ = 700 → circa 160 bar.

## 2. Ionizzazione, vuoto e trasporto degli ioni (capitoli 6, 7, 8)

### Ionizzazione elettronica e chimica

```html
<span class="eq">M + e<sup>−</sup> (70 eV) → M<sup>+•</sup> + 2 e<sup>−</sup>, &nbsp;&nbsp; λ<sub>e</sub> = <span class="frac"><span><i>h</i></span><span>√(2 <i>m</i><sub>e</sub> <i>E</i>)</span></span> ≈ <span class="frac"><span>1,226 nm</span><span>√(<i>E</i>/eV)</span></span><span class="no">(7.1)</span></span>
```

A 70 eV la lunghezza d'onda di de Broglie dell'elettrone (≈ 0,15 nm) è confrontabile con le lunghezze di legame: il trasferimento di energia è massimo e gli spettri sono riproducibili da uno strumento all'altro (librerie). Le energie di ionizzazione delle molecole organiche sono 8–11 eV: l'eccesso, in media qualche eV, finisce come energia interna e frammenta lo ione radicale (catione a elettroni dispari).

```html
<span class="eq">BH<sup>+</sup> + M → MH<sup>+</sup> + B, &nbsp;&nbsp; −Δ<i>H</i> = PA(M) − PA(B)<span class="no">(7.2)</span></span>
```

```html
<p>Ionizzazione chimica: il trasferimento di protone avviene se l'affinità protonica di M supera quella della base coniugata del gas reagente; la differenza è l'energia in eccesso che può frammentare MH<sup>+</sup>. PA: CH<sub>4</sub> 544, isobutene 802, NH<sub>3</sub> 854 kJ/mol: metano è il reagente «duro», ammoniaca il più «morbido» (spesso solo [M+NH<sub>4</sub>]<sup>+</sup>).</p>
```

### Elettrospray

```html
<span class="eq"><i>q</i><sub>R</sub> = 8π (ε<sub>0</sub> γ <i>R</i><sup>3</sup>)<sup>1/2</sup><span class="no">(6.1)</span></span>
```

Limite di Rayleigh: carica massima di una goccia di raggio *R* e tensione superficiale γ. Il solvente evapora, *R* cala a carica costante, la repulsione supera la tensione superficiale e la goccia esplode in goccioline più piccole (fissione coulombiana). Solventi a γ bassa (metanolo, acetonitrile) facilitano lo spray rispetto all'acqua pura.

```html
<span class="eq"><i>z</i> = <span class="frac"><span><i>p</i><sub>2</sub> − 1,007</span><span><i>p</i><sub>1</sub> − <i>p</i><sub>2</sub></span></span>, &nbsp;&nbsp; <i>M</i> = <i>z</i> (<i>p</i><sub>1</sub> − 1,007)<span class="no">(6.4)</span></span>
```

```html
<p>Ioni multicarica [M+<i>z</i>H]<sup><i>z</i>+</sup>: da due picchi adiacenti della serie (<i>p</i><sub>1</sub> &gt; <i>p</i><sub>2</sub>, cariche <i>z</i> e <i>z</i>+1) si ricavano carica e massa. In un singolo gruppo isotopico la distanza fra picchi è 1/<i>z</i>.</p>
```

```html
<span class="eq">ME (%) = <span class="frac"><span>area in matrice (dopo l'estrazione)</span><span>area in solvente</span></span> × 100 − 100<span class="no">(6.3)</span></span>
```

Effetto matrice: negativo = soppressione (i componenti della matrice competono per la carica e la superficie delle gocce, equazione 6.2), positivo = esaltazione. Si corregge con standard interni marcati isotopicamente o calibrazione in matrice.

### Vuoto

```html
<span class="eq">λ = <span class="frac"><span><i>k</i><sub>B</sub><i>T</i></span><span>√2 π <i>d</i><sup>2</sup> <i>p</i></span></span> &nbsp;&nbsp; ⇒ &nbsp; per N<sub>2</sub> a 25 °C: &nbsp; λ (cm) ≈ <span class="frac"><span>6,8·10<sup>−3</sup></span><span><i>p</i> (mbar)</span></span><span class="no">(8.1)</span></span>
```

Cammino libero medio: a pressione atmosferica ≈ 70 nm, a 1 mbar ≈ 70 µm, a 10^−5^ mbar ≈ 7 m. Nell'analizzatore (lungo 20 cm) λ deve essere molto più lungo del percorso: per questo si scende a 10^−5^ mbar, e servono stadi di pompaggio differenziale fra la sorgente a pressione atmosferica e il quadrupolo.

```html
<span class="eq"><i>zeU</i> = ½ <i>m</i> <i>v</i><sup>2</sup> &nbsp;&nbsp; ⇒ &nbsp; <i>v</i> = √(2<i>zeU</i>/<i>m</i>)</span>
```

Uno ione accelerato da una differenza di potenziale *U* acquista energia cinetica *zeU* (in eV: *z* × *U*), uguale per tutti gli ioni con la stessa carica; la velocità invece dipende da *m*/*z*. È la base del TOF e del settore magnetico.

## 3. Analizzatori (capitoli 9, 10, 11)

### Quadrupolo

```html
<span class="eq">Φ<sub>0</sub>(<i>t</i>) = <i>U</i> − <i>V</i> cos Ω<i>t</i>, &nbsp;&nbsp; Φ(<i>x</i>, <i>y</i>, <i>t</i>) = Φ<sub>0</sub>(<i>t</i>) <span class="frac"><span><i>x</i><sup>2</sup> − <i>y</i><sup>2</sup></span><span><i>r</i><sub>0</sub><sup>2</sup></span></span><span class="no">(9.1–9.2)</span></span>
```

Due coppie di barre con tensione continua *U* e radiofrequenza di ampiezza *V* e frequenza Ω/2π (≈ 1 MHz). Il campo è lineare in *x* e *y*: i moti nelle due direzioni sono indipendenti.

```html
<span class="eq"><i>a</i> = <span class="frac"><span>8 <i>zeU</i></span><span><i>m</i> <i>r</i><sub>0</sub><sup>2</sup> Ω<sup>2</sup></span></span>, &nbsp;&nbsp; <i>q</i> = <span class="frac"><span>4 <i>zeV</i></span><span><i>m</i> <i>r</i><sub>0</sub><sup>2</sup> Ω<sup>2</sup></span></span>, &nbsp;&nbsp; <span class="frac"><span><i>a</i></span><span><i>q</i></span></span> = <span class="frac"><span>2<i>U</i></span><span><i>V</i></span></span><span class="no">(9.4–9.5, 9.7)</span></span>
```

Parametri di Mathieu: la stabilità della traiettoria dipende **solo** da (*a*, *q*). A *U*/*V* fisso tutti gli ioni stanno sulla stessa retta passante per l'origine del diagramma di stabilità (la «linea di scansione»), ciascuno in un punto proporzionale a 1/(*m*/*z*).

```html
<span class="eq">vertice della regione stabile: &nbsp; <i>q</i> = 0,706, &nbsp; <i>a</i> = 0,237 &nbsp; (<i>U</i>/<i>V</i> = 0,168); &nbsp;&nbsp; <span class="frac"><span><i>m</i></span><span><i>z</i></span></span> = <span class="frac"><span>4 <i>e</i></span><span><i>q</i>* <i>r</i><sub>0</sub><sup>2</sup> Ω<sup>2</sup></span></span> <i>V</i><span class="no">(9.8–9.9)</span></span>
```

```html
<p>Passa solo lo ione la cui <i>q</i> cade nel piccolo tratto della linea di scansione dentro la regione stabile, vicino al vertice (<i>q</i>* ≈ 0,706). La massa trasmessa è <b>proporzionale a <i>V</i></b>: si scandisce lo spettro aumentando <i>V</i> (e <i>U</i> insieme). Esempio con <i>r</i><sub>0</sub> = 4 mm e 1 MHz: <i>m</i>/<i>z</i> 300 richiede <i>V</i> ≈ 350 V (circa 1,16 V per unità di <i>m</i>/<i>z</i>). Più <i>U</i>/<i>V</i> si avvicina a 0,168, più la finestra si stringe: più risoluzione, meno trasmissione.</p>
```

```html
<span class="eq"><i>a</i> = 0: stabili tutti gli ioni con 0 &lt; <i>q</i> &lt; 0,908; &nbsp;&nbsp; <i>D</i> ≈ <span class="frac"><span><i>q V</i></span><span>4</span></span> (in volt, <i>q</i> ≲ 0,4)<span class="no">(9.10)</span></span>
```

Quadrupolo solo RF (q0, cella di collisione q2): guida di ioni senza selezione, con un **taglio di bassa massa** (gli ioni con *q* > 0,908 sono persi). *D* è la profondità della buca di pseudopotenziale che rifocalizza gli ioni sull'asse dopo gli urti.

### Triplo quadrupolo e CID

```html
<span class="eq"><i>E</i><sub>cm</sub> = <i>E</i><sub>lab</sub> <span class="frac"><span><i>m</i><sub>gas</sub></span><span><i>m</i><sub>gas</sub> + <i>m</i><sub>ione</sub></span></span><span class="no">(10.1)</span></span>
```

Energia del centro di massa: la parte dell'energia di collisione che può diventare energia interna in un singolo urto. Ione di *m*/*z* 300 a 30 eV: con N~2~ (28) 2,6 eV, con Ar (40) 3,5 eV. Ioni più pesanti, a parità di CE, ricevono meno energia per urto; nella cella ci sono decine di urti e l'energia si accumula (attivazione lenta).

```html
<span class="eq">tempo di ciclo = Σ (dwell + pausa) &nbsp;&nbsp; ⇒ &nbsp; punti per picco = <span class="frac"><span>larghezza del picco cromatografico</span><span>tempo di ciclo</span></span> ≥ 10–15</span>
```

In MRM il quadrupolo «guarda» una transizione alla volta (duty cycle ≈ 100% per transizione, ma diviso fra le transizioni). 20 transizioni × (20 ms + 5 ms) = 0,5 s: un picco largo 6 s ha 12 punti, appena sufficienti per quantificare.

### Tempo di volo (TOF)

```html
<span class="eq"><i>t</i> = <i>L</i> <span class="frac"><span>1</span><span>√(2<i>zeU</i>)</span></span> √<i>m</i>, &nbsp;&nbsp; <span class="frac"><span><i>m</i></span><span><i>z</i></span></span> = <span class="frac"><span>2<i>eU</i></span><span><i>L</i><sup>2</sup></span></span> <i>t</i><sup>2</sup>, &nbsp;&nbsp; <i>R</i> = <span class="frac"><span><i>m</i></span><span>Δ<i>m</i></span></span> = <span class="frac"><span><i>t</i></span><span>2Δ<i>t</i></span></span><span class="no">(11.3)</span></span>
```

Il tempo di volo cresce con √(*m*/*z*). Esempio: *L* = 1 m, *U* = 20 kV, *m*/*z* 300 → *t* ≈ 8,8 µs. Il fattore 2 in *R* viene da *m* ∝ *t*^2^. Il reflectron corregge la dispersione di energia cinetica iniziale e allunga il percorso: *R* 20 000–60 000.

### Orbitrap e FT-ICR

```html
<span class="eq">ω<sub><i>z</i></sub> = √(<i>k</i> · <i>z</i>/<i>m</i>) &nbsp;&nbsp; (Orbitrap), &nbsp;&nbsp;&nbsp; <i>f</i><sub>c</sub> = <span class="frac"><span><i>zeB</i></span><span>2π<i>m</i></span></span> &nbsp;&nbsp; (FT-ICR)<span class="no">(11.4)</span></span>
```

```html
<p>Nell'Orbitrap la frequenza delle oscillazioni assiali dipende solo da <i>m</i>/<i>z</i> (non dall'energia): <i>m</i>/<i>z</i> ∝ 1/ω<sup>2</sup>. Nell'FT-ICR la frequenza di ciclotrone è ∝ <i>B</i>/(<i>m</i>/<i>z</i>): con 7 T, <i>m</i>/<i>z</i> 300 → ≈ 360 kHz. In entrambi la risoluzione cresce con la durata del transiente <i>T</i> (Δ<i>f</i> ≈ 1/<i>T</i>) e cala con la massa: <i>R</i> ∝ 1/√(<i>m</i>/<i>z</i>) nell'Orbitrap (per questo si dichiara «a <i>m</i>/<i>z</i> 200»), <i>R</i> ∝ 1/(<i>m</i>/<i>z</i>) nell'FT-ICR.</p>
```

### Settore magnetico e trappola ionica

```html
<span class="eq"><span class="frac"><span><i>m</i></span><span><i>z</i></span></span> = <span class="frac"><span><i>B</i><sup>2</sup> <i>r</i><sup>2</sup> <i>e</i></span><span>2<i>U</i></span></span> &nbsp;&nbsp; (settore magnetico), &nbsp;&nbsp;&nbsp; <i>q</i><sub><i>z</i></sub> = <span class="frac"><span>8 <i>zeV</i></span><span><i>m</i> (<i>r</i><sub>0</sub><sup>2</sup> + 2<i>z</i><sub>0</sub><sup>2</sup>) Ω<sup>2</sup></span></span> &nbsp;&nbsp; (trappola di Paul)</span>
```

```html
<p>Settore: forza di Lorentz = forza centripeta (<i>zevB</i> = <i>mv</i><sup>2</sup>/<i>r</i>) con <i>v</i> dall'accelerazione <i>zeU</i> = ½<i>mv</i><sup>2</sup>; a <i>r</i> e <i>U</i> fissi si scandisce <i>B</i>. Esempio: 1 T, <i>r</i> = 0,3 m, 8 kV → <i>m</i>/<i>z</i> ≈ 540. Trappola: stesse equazioni di Mathieu del quadrupolo; con <i>U</i> = 0 gli ioni vengono espulsi in ordine di <i>m</i>/<i>z</i> aumentando <i>V</i> fino a <i>q</i><sub><i>z</i></sub> = 0,908 (instabilità selettiva in massa).</p>
```

### Risoluzione e accuratezza

```html
<span class="eq"><i>R</i> = <span class="frac"><span><i>m</i></span><span>Δ<i>m</i><sub>FWHM</sub></span></span>, &nbsp;&nbsp; errore (ppm) = <span class="frac"><span><i>m</i><sub>mis</sub> − <i>m</i><sub>teor</sub></span><span><i>m</i><sub>teor</sub></span></span> × 10<sup>6</sup><span class="no">(11.1–11.2)</span></span>
```

```html
<p>Per separare due ioni di massa <i>m</i><sub>1</sub> e <i>m</i><sub>2</sub> serve almeno <i>R</i> ≈ <i>m</i>/(<i>m</i><sub>2</sub> − <i>m</i><sub>1</sub>). CO<sup>+•</sup> (27,9949) e N<sub>2</sub><sup>+•</sup> (28,0061): <i>R</i> ≈ 2500. A <i>m</i>/<i>z</i> 300, 5 ppm = 1,5 mDa. Il quadrupolo ha Δ<i>m</i> ≈ 0,7 costante (<i>R</i> cresce con la massa: «risoluzione unitaria»); gli analizzatori ad alta risoluzione hanno <i>R</i> da 10<sup>4</sup> a 10<sup>6</sup>.</p>
```

```html
<table>
<tr><th>Analizzatore</th><th>Grandezza misurata</th><th>Dipendenza da m/z</th><th>R tipica (FWHM)</th><th>Accuratezza</th></tr>
<tr><td>Quadrupolo, trappola</td><td>stabilità (<i>a</i>, <i>q</i>)</td><td><i>m</i>/<i>z</i> ∝ <i>V</i></td><td>Δ<i>m</i> ≈ 0,7 (unitaria)</td><td>0,1–0,2 u</td></tr>
<tr><td>Settore magnetico</td><td>raggio di curvatura</td><td><i>m</i>/<i>z</i> ∝ <i>B</i><sup>2</sup></td><td>fino a 10<sup>4</sup>–10<sup>5</sup> (doppia focalizzazione)</td><td>&lt; 5 ppm</td></tr>
<tr><td>TOF (reflectron)</td><td>tempo di volo</td><td><i>t</i> ∝ √(<i>m</i>/<i>z</i>)</td><td>2–6·10<sup>4</sup></td><td>1–5 ppm</td></tr>
<tr><td>Orbitrap</td><td>frequenza assiale</td><td><i>f</i> ∝ 1/√(<i>m</i>/<i>z</i>)</td><td>1,5·10<sup>4</sup>–5·10<sup>5</sup> a <i>m</i>/<i>z</i> 200</td><td>1–3 ppm</td></tr>
<tr><td>FT-ICR</td><td>frequenza di ciclotrone</td><td><i>f</i> ∝ 1/(<i>m</i>/<i>z</i>)</td><td>10<sup>5</sup>–10<sup>7</sup></td><td>&lt; 1 ppm</td></tr>
</table>
```

## 4. Leggere uno spettro (capitoli 13, 15)

```html
<span class="eq">RDB = <i>c</i> − <span class="frac"><span><i>h</i> + <i>x</i></span><span>2</span></span> + <span class="frac"><span><i>n</i></span><span>2</span></span> + 1<span class="no">(13.1)</span></span>
```

Anelli più doppi legami da C~*c*~H~*h*~N~*n*~O~*o*~X~*x*~ (X = alogeni; O e S non contano; Si come C, P come N). Intero per una molecola o uno ione a elettroni dispari (M^+•^), semintero per uno ione a elettroni pari: è il controllo della notazione dei frammenti. Benzene 4, un C=O 1.

```html
<p><b>Regola dell'azoto</b>: massa nominale dispari ⇔ numero dispari di atomi di N (per la molecola neutra e per M<sup>+•</sup>; per i frammenti a elettroni pari vale il contrario).</p>
```

```html
<span class="eq"><i>n</i><sub>C</sub> ≈ <span class="frac"><span>100 · <i>I</i>(M+1)/<i>I</i>(M) − 0,37 <i>n</i><sub>N</sub></span><span>1,1</span></span>, &nbsp;&nbsp; <i>I</i>(M+2)/<i>I</i>(M) ≈ <span class="frac"><span>(1,1 <i>n</i><sub>C</sub>)<sup>2</sup></span><span>200</span></span> + 0,2 <i>n</i><sub>O</sub> + 4,4 <i>n</i><sub>S</sub> &nbsp;(%)<span class="no">(13.2)</span></span>
```

Gli isotopi pesanti: ^13^C 1,1% per C, ^15^N 0,37%, ^18^O 0,2%, ^34^S 4,4%, ^30^Si 3,4% (più ^29^Si 5,1% su M+1). Cl: M+2 ≈ 32% per ogni Cl (Cl~2~ 100:64:10); Br: M+2 ≈ 97% (Br~2~ 1:2:1). F, I, P sono monoisotopici.

```html
<span class="eq"><span class="frac"><span><i>M</i></span><span>13</span></span> = <i>n</i> + <span class="frac"><span><i>r</i></span><span>13</span></span> &nbsp;⇒&nbsp; C<sub><i>n</i></sub>H<sub><i>n</i>+<i>r</i></sub>, &nbsp; RDB = <span class="frac"><span><i>n</i> − <i>r</i> + 2</span><span>2</span></span><span class="no">(13.3)</span></span>
```

Regola del 13: formula idrocarburica di partenza dalla sola massa nominale; poi si sostituisce CH~4~ con O (stessa massa 16), CH~2~ con N (14), C~2~H~8~ con S... aggiornando RDB. M = 128: 128/13 = 9 resto 11 → C~9~H~20~ (RDB 0) oppure C~10~H~8~ (naftalene, RDB 7).

```html
<span class="eq">massa di Kendrick = <i>m</i> × <span class="frac"><span>14,00000</span><span>14,01565</span></span>, &nbsp;&nbsp; KMD = massa nominale − massa di Kendrick</span>
```

Riportata alla scala in cui CH~2~ pesa esattamente 14: gli omologhi di una serie (che differiscono per CH~2~) hanno lo stesso difetto di massa di Kendrick e si allineano in orizzontale.

```html
<table>
<tr><th>Massa esatta</th><td>H 1,007825</td><td>C 12,000000</td><td>N 14,003074</td><td>O 15,994915</td><td>F 18,998403</td></tr>
<tr><th></th><td>P 30,973762</td><td>S 31,972071</td><td>Cl 34,968853</td><td>Br 78,918338</td><td>I 126,904473</td></tr>
<tr><th>Elettrone, protone</th><td colspan="2">e<sup>−</sup> 0,000549</td><td colspan="3">H<sup>+</sup> 1,007276 (massa di [M+H]<sup>+</sup> = M + 1,007276)</td></tr>
</table>
```

## 5. Cinetica e dati quantitativi (capitoli 2, 12)

```html
<span class="eq"><i>C</i>(<i>t</i>) = <i>C</i><sub>0</sub> e<sup>−<i>k</i><sub>app</sub><i>t</i></sup> &nbsp;&nbsp;⇔&nbsp;&nbsp; ln(<i>C</i><sub>0</sub>/<i>C</i>) = <i>k</i><sub>app</sub> <i>t</i>, &nbsp;&nbsp; <i>t</i><sub>1/2</sub> = ln 2 / <i>k</i><sub>app</sub><span class="no">(2.3)</span></span>
```

Pseudo-primo ordine (Langmuir–Hinshelwood a bassa concentrazione, equazione 2.2): la retta di ln(*C*~0~/*C*) contro il tempo dà *k*~app~. Per un prodotto di trasformazione intermedio A → B → C l'area di B sale, passa per un massimo a *t*~max~ = ln(*k*~2~/*k*~1~)/(*k*~2~ − *k*~1~) e poi cala (equazione 2.4).

```html
<span class="eq">LOD ≈ 3,3 <span class="frac"><span><i>s</i></span><span><i>b</i></span></span>, &nbsp;&nbsp; LOQ ≈ 10 <span class="frac"><span><i>s</i></span><span><i>b</i></span></span><span class="no">(12.1)</span></span>
```

```html
<p><i>s</i> deviazione standard del segnale del bianco (o dei residui della retta), <i>b</i> pendenza della retta di taratura. In pratica: segnale/rumore 3 per rivelare, 10 per quantificare.</p>
```

## Verifica

??? Un quadrupolo trasmette m/z 300 con V = 350 V. Che V serve per m/z 450, a parità di U/V?
La massa trasmessa è proporzionale a V (equazione 9.9): V = 350 × 450/300 = 525 V. U cresce nella stessa proporzione, così il rapporto U/V (e quindi la risoluzione relativa al vertice) resta lo stesso.

??? Raddoppio la lunghezza della colonna. Di quanto migliora la risoluzione? E il tempo di analisi?
N raddoppia, R~s~ ∝ √N cresce di √2 ≈ 1,4 volte; il tempo di analisi raddoppia (e in HPLC anche la pressione). Cambiare la selettività (fase stazionaria, solvente, temperatura) è quasi sempre più efficace.

??? Perché lo stesso ione si frammenta meno in una cella con N₂ che con Ar alla stessa energia di collisione?
Per l'equazione 10.1 l'energia nel centro di massa è proporzionale a m~gas~/(m~gas~ + m~ione~): N~2~ (28) trasferisce meno energia per urto di Ar (40).

??? Un TOF misura uno ione a 20,0 µs. A che tempo arriva uno ione di m/z quattro volte più grande?
t ∝ √(m/z): √4 = 2, quindi a 40,0 µs.

??? Uno spettro EI ha M^+•^ a m/z 136, M+1 al 9,9% di M e nessun M+2 significativo. Quanti carboni? Che formule sono possibili?
Massa pari: nessun N o un numero pari. n~C~ ≈ 9,9/1,1 = 9. Regola del 13: 136/13 = 10 resto 6 → C~10~H~16~ (RDB 3, per esempio un monoterpene); con un O al posto di CH~4~: C~9~H~12~O (RDB 4, compatibile con 9 carboni). L'M+1 favorisce 9 carboni; il resto lo dicono i frammenti.
