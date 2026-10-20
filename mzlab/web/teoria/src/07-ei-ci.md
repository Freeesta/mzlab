<!-- script: sim-ei.js -->
# Ionizzazione elettronica (EI) e chimica (CI) {kicker: Capitolo 7}

{.lead} Nella GC-MS le molecole arrivano già in fase gas e nel vuoto: si ionizzano bombardandole con elettroni. Ne nascono radicali cationi con molta energia interna, che si frammentano in modo riproducibile. È l'origine degli spettri EI che imparerete a risolvere nei capitoli 15 e 16.

## La sorgente EI

```html
<figure>
<svg viewBox="0 0 860 260" role="img" aria-label="Schema di una sorgente a impatto elettronico: camera di ionizzazione, filamento, fascio di elettroni, repeller e lenti" aria-describedby="fd-07-0">
<style>.t{font:12.5px system-ui,sans-serif;fill:#24231f}.s{font:11.5px system-ui,sans-serif;fill:#6b675c}</style>
<rect x="250" y="60" width="300" height="140" rx="8" fill="#f8fafc" stroke="#9b978c" stroke-width="1.4"/>
<text class="s" x="400" y="80" text-anchor="middle">camera di ionizzazione (~10⁻⁵ Torr, 200-250 °C)</text>
<path d="M150 130 H250" stroke="#2b5c8a" stroke-width="3"/><text class="t" x="150" y="118">dalla colonna GC</text>
<path d="M400 20 Q392 30 400 40 Q408 50 400 60" fill="none" stroke="#c2410c" stroke-width="3"/><text class="t" x="412" y="34">filamento (W o Re), −70 V</text>
<path d="M400 62 V196" stroke="#c2410c" stroke-width="1.6" stroke-dasharray="5 4"/><text class="s" x="408" y="175">fascio di elettroni</text>
<rect x="380" y="200" width="40" height="14" fill="#c2410c" opacity=".7"/><text class="s" x="430" y="212">trappola (collettore)</text>
<rect x="262" y="95" width="10" height="70" fill="#7c3aed" opacity=".7"/><text class="s" x="246" y="250">repeller (+): spinge gli ioni</text>
<path d="M272 160 L250 240" stroke="#9b978c" stroke-width=".8"/>
<circle cx="360" cy="130" r="6" fill="#2b5c8a"/><circle cx="440" cy="118" r="6" fill="#2b5c8a"/><circle cx="455" cy="145" r="5" fill="#c2410c"/>
<text class="s" x="452" y="166">M⁺•</text>
<rect x="560" y="85" width="10" height="90" fill="#9b978c"/><rect x="590" y="85" width="10" height="90" fill="#9b978c"/><rect x="620" y="85" width="10" height="90" fill="#9b978c"/>
<text class="s" x="595" y="78" text-anchor="middle">lenti di estrazione</text>
<path d="M470 130 H700" stroke="#c2410c" stroke-width="2.4" marker-end="url(#ar)"/><defs><marker id="ar" markerWidth="8" markerHeight="8" refX="6" refY="4" orient="auto"><path d="M0 0 L8 4 L0 8 z" fill="#c2410c"/></marker></defs>
<text class="t" x="710" y="134">all'analizzatore</text>
<text class="s" x="560" y="225">magneti: gli elettroni percorrono una spirale più lunga</text>
</svg><details class="figdesc" id="fd-07-0"><summary>Descrizione della figura</summary><p>Una camera (~10⁻⁵ Torr, 200-250 °C) riceve a sinistra l'ingresso dalla colonna GC. In alto il filamento (W o Re, −70 V) emette il fascio di elettroni, in linea tratteggiata, verso la trappola (collettore) in basso. A sinistra il repeller (+) spinge gli ioni; M⁺• esce verso destra, attraverso le lenti di estrazione, all'analizzatore. Nota: i magneti allungano la spirale degli elettroni.</p></details>
<figcaption><b>Figura 7.1</b> Sorgente a impatto elettronico (schema non in scala). Gli elettroni emessi dal filamento caldo sono accelerati da una differenza di potenziale di 70 V e attraversano la camera fino alla trappola; le molecole che escono dalla colonna li incontrano, una piccola frazione (circa una su mille o meno) si ionizza e gli ioni sono spinti verso l'analizzatore.</figcaption>
</figure>
```

Tutto avviene nel **vuoto** (10^−5^-10^−6^ Torr): il cammino libero medio è di metri (capitolo 8), quindi uno ione non urta altre molecole prima di uscire. Le reazioni che vede sono solo **unimolecolari**: lo ione si frammenta da solo, con l'energia che ha ricevuto. È il motivo per cui gli spettri EI sono «firme» della molecola e si possono raccogliere in librerie.

## Che cosa succede alla molecola

```html
<span class="eq">M + e<sup>−</sup> (70 eV) → M<sup>+•</sup> + 2 e<sup>−</sup><span class="no">(7.1)</span></span>
```

L'elettrone non «colpisce» la molecola: le passa vicino in circa 10^−16^ s e il suo campo elettrico perturba gli elettroni molecolari. Se l'energia trasferita supera l'**energia di ionizzazione** (IE, 7-12 eV per la maggior parte delle molecole organiche) un elettrone viene espulso. In quel tempo i nuclei non si muovono (principio di Franck-Condon: una vibrazione dura ~10^−13^ s).

- Il prodotto è lo **ione molecolare M^+•^**: un **radicale catione**, con un elettrone spaiato e una carica positiva. È uno ione a **elettroni dispari** (OE): tutta la logica degli spettri EI parte da qui (capitolo 15).
- Si perde di preferenza l'elettrone meno legato: quello di un doppietto libero su un eteroatomo (n), poi di un legame π, infine di un legame σ. Il sito dove si trova la carica e il radicale guida la frammentazione.
- Insieme alla ionizzazione la molecola riceve energia: il trasferimento medio è di qualche eV, con una coda fino a 10-20 eV. Un legame C–C vale circa 3,6 eV (83 kcal/mol): molti ioni hanno energia per rompersi.

### Perché proprio 70 eV

Un elettrone ha una lunghezza d'onda di de Broglie λ = *h*/(*m*~e~*v*), cioè λ (nm) ≈ 1,226/√*E*(eV). A 70 eV λ ≈ 1,5 Å, confrontabile con le distanze di legame: è la condizione in cui il trasferimento di energia è più efficace. La resa di ionizzazione cresce dalla soglia, ha un massimo largo fra 50 e 100 eV e poi scende lentamente.

- Sul **plateau** piccole variazioni dell'energia non cambiano lo spettro: gli spettri di strumenti diversi si somigliano e si possono confrontare con le **librerie**.
- A energie basse (12-20 eV) si formano meno ioni, con meno energia interna: meno frammenti e ione molecolare più intenso, ma sensibilità molto più bassa e spettri non confrontabili con le librerie.

```html
<div class="sim" id="sim-ei-eff"><h4>Efficienza di ionizzazione e lunghezza d'onda dell'elettrone</h4>
<p class="note">La fascia verde è il plateau dove si lavora. Sotto l'energia di ionizzazione non si forma nessuno ione.</p></div>
```

### Energia interna e frammentazione

Gli ioni M^+•^ nascono con una **distribuzione** di energie interne P(E), non con un valore unico. L'energia si ridistribuisce in tutte le vibrazioni in meno di un picosecondo; poi lo ione si comporta come una molecola «calda» isolata e si rompe dove è più probabile, con velocità k(E) che dipendono dall'energia in eccesso sopra la soglia e dalla struttura dello stato di transizione (teoria quasi-equilibrio, QET, o RRKM, la stessa della CID nel capitolo 14):

- gli ioni con poca energia (sotto la soglia più bassa) restano **M^+•^**;
- quelli con un po' di energia seguono soprattutto i **riarrangiamenti** (stato di transizione ordinato, soglia bassa ma fattore di frequenza piccolo): per esempio il riarrangiamento di McLafferty;
- quelli con molta energia preferiscono le **scissioni semplici** (stato di transizione «lasco», fattore di frequenza grande) e i frammenti si frammentano ancora (frammentazioni consecutive);
- conta solo ciò che succede nel tempo in cui lo ione resta nella sorgente (~1 µs): uno ione che si rompe più tardi, durante il volo, è uno **ione metastabile**.

```html
<details class="poe"><summary>Prima di provare: abbassando l'energia degli elettroni da 70 a 12 eV, quale picco crescerà di più?</summary>
<p>Lo ione molecolare, e fra i frammenti quello del riarrangiamento a soglia più bassa (m/z 58 del 2-esanone), mentre le scissioni semplici (m/z 43, 85) calano. È il principio della «EI a bassa energia» e del perché i riarrangiamenti sono più importanti negli ioni «freddi».</p></details>
```

```html
<div class="sim" id="sim-ei-frag"><h4>Dall'energia interna allo spettro: un chetone modello</h4>
<p class="note">A sinistra la distribuzione dell'energia interna degli ioni M<sup>+•</sup> del 2-esanone (m/z 100) e le soglie di tre canali; a destra lo spettro che ne risulta dopo 1 µs nella sorgente. Le scissioni α e il riarrangiamento di McLafferty sono spiegati nel capitolo 15.</p></div>
```

### Che cosa si vede in uno spettro EI

```html
<ul>
<li><b>Ione molecolare</b> M<sup>+•</sup>: intenso per molecole stabili (aromatici, sistemi coniugati), debole o assente per alcoli terziari, alcani molto ramificati, molecole con legami deboli.</li>
<li><b>Frammenti</b>: ioni a elettroni pari (cationi) nati da scissioni semplici, e ioni a elettroni dispari nati da riarrangiamenti.</li>
<li><b>Picchi isotopici</b> di ogni ione (capitolo 13) e, per molecole con alogeni, gruppi caratteristici.</li>
<li>Ioni a <b>carica doppia</b> (m/z dimezzato, anche semi-intero) negli aromatici policiclici; ioni metastabili come piccoli picchi larghi a m/z non interi negli strumenti a settore.</li>
</ul>
```

## Le librerie di spettri EI

Poiché a 70 eV gli spettri sono riproducibili, esistono raccolte di centinaia di migliaia di spettri di riferimento: la libreria NIST/EPA/NIH (oltre 300 000 composti, con gli indici di ritenzione del capitolo 5) e la Wiley Registry. Il software confronta lo spettro misurato con ogni spettro di libreria e calcola un punteggio di somiglianza (prodotto scalare delle intensità pesate per la massa; «match factor» da 0 a 999). Un punteggio alto **non è un'identificazione**: isomeri di posizione hanno spettri quasi uguali, lo spettro può contenere ioni di un coeluito o del fondo, e il composto può non essere in libreria. Si conferma con l'indice di ritenzione e, quando possibile, con lo standard. Le librerie aperte (MassBank, che fornisce gli spettri della Pratica) sono più piccole ma liberamente consultabili.

## Ionizzazione chimica (CI)

Nella sorgente si introduce un **gas reagente** a pressione relativamente alta (~1 Torr), mille volte più dell'analita. Gli elettroni ionizzano il gas; gli ioni del gas reagiscono fra loro e formano **ioni reagenti** stabili, che a loro volta ionizzano l'analita con reazioni ione-molecola a bassa energia:

```html
<span class="eq">CH<sub>4</sub><sup>+•</sup> + CH<sub>4</sub> → CH<sub>5</sub><sup>+</sup> + CH<sub>3</sub><sup>•</sup>, &nbsp;&nbsp; CH<sub>5</sub><sup>+</sup> + M → [M+H]<sup>+</sup> + CH<sub>4</sub><span class="no">(7.2)</span></span>
```

```html
<p>L'energia in eccesso dello ione [M+H]<sup>+</sup> è circa la differenza fra le <b>affinità protoniche</b> (PA) dell'analita e della base coniugata del gas: più è piccola, più la ionizzazione è soffice.</p>
```

```html
<table>
<tr><th>Gas reagente</th><th>Ioni reagenti</th><th>Ioni dell'analita</th><th>Carattere</th></tr>
<tr><td>Metano</td><td>CH<sub>5</sub><sup>+</sup>, C<sub>2</sub>H<sub>5</sub><sup>+</sup>, C<sub>3</sub>H<sub>5</sub><sup>+</sup></td><td>[M+H]<sup>+</sup>, [M+C<sub>2</sub>H<sub>5</sub>]<sup>+</sup> (M+29), [M+C<sub>3</sub>H<sub>5</sub>]<sup>+</sup> (M+41), talvolta [M−H]<sup>+</sup></td><td>La più «dura» delle CI: un po' di frammentazione</td></tr>
<tr><td>Isobutano</td><td>t-C<sub>4</sub>H<sub>9</sub><sup>+</sup></td><td>[M+H]<sup>+</sup>, [M+C<sub>4</sub>H<sub>9</sub>]<sup>+</sup></td><td>Soffice, poca frammentazione</td></tr>
<tr><td>Ammoniaca</td><td>NH<sub>4</sub><sup>+</sup></td><td>[M+H]<sup>+</sup> per le basi più forti dell'NH<sub>3</sub>, [M+NH<sub>4</sub>]<sup>+</sup> (M+18) per le altre</td><td>La più soffice e selettiva</td></tr>
</table>
```

```html
<div class="sim" id="sim-ci"><h4>Quanto è soffice la CI?</h4>
<p class="note">Scegli il gas e l'affinità protonica dell'analita: la riga sotto calcola l'entalpia del trasferimento di protone e ne deduce che cosa si vedrà.</p></div>
```

```html
<p><b>CI negativa (NCI o ECNI)</b>: il gas reagente (metano) rallenta gli elettroni fino all'energia termica; molecole molto elettronegative (polialogenate, nitroaromatiche) li catturano formando M<sup>−•</sup> o ioni come Cl<sup>−</sup>. È estremamente sensibile e selettiva (fg) per pesticidi organoclorurati, PCB e ritardanti di fiamma bromurati: lo stesso principio del rivelatore ECD (capitolo 5).</p>
```

```html
<div class="box key"><b>EI e CI insieme</b>
<p>Lo spettro EI dice <em>com'è fatta</em> la molecola (frammenti), ma lo ione molecolare può mancare; lo spettro CI dice <em>quanto pesa</em> (M+1, M+29, M+41 con il metano: tre picchi distanziati di 28 e 12 confermano M). Il procedimento di McLafferty (capitolo 15, passo 2) suggerisce proprio questo controllo.</p></div>
```

## Tutte le sorgenti a confronto

```html
<table>
<tr><th>Sorgente</th><th>Dove</th><th>Ione tipico</th><th>Energia</th><th>Accoppiata a</th><th>Per che cosa</th></tr>
<tr><td><b>EI</b></td><td>vuoto</td><td>M<sup>+•</sup> (elettroni dispari) + molti frammenti</td><td>dura</td><td>GC</td><td>volatili, termostabili, apolari-medi; librerie</td></tr>
<tr><td><b>CI</b> (+/−)</td><td>~1 Torr</td><td>[M+H]<sup>+</sup>, addotti; M<sup>−•</sup> in NCI</td><td>soffice</td><td>GC</td><td>massa molecolare; tracce di alogenati (NCI)</td></tr>
<tr><td><b>ESI</b></td><td>pressione atmosferica</td><td>[M+H]<sup>+</sup>, [M−H]<sup>−</sup>, addotti, ioni multicarica</td><td>molto soffice</td><td>LC</td><td>polari, ionizzabili, termolabili, grandi (capitolo 6)</td></tr>
<tr><td><b>APCI</b></td><td>pressione atmosferica</td><td>[M+H]<sup>+</sup>, [M−H]<sup>−</sup> (CI in fase gas con gli ioni del solvente)</td><td>soffice</td><td>LC</td><td>meno polari, piccoli, termostabili; meno effetto matrice</td></tr>
<tr><td><b>APPI</b></td><td>pressione atmosferica</td><td>M<sup>+•</sup> o [M+H]<sup>+</sup> (fotoni da 10 eV, spesso con un dopante)</td><td>soffice</td><td>LC</td><td>apolari: IPA, steroidi</td></tr>
<tr><td>MALDI</td><td>vuoto o atmosferica</td><td>[M+H]<sup>+</sup> (una sola carica)</td><td>soffice</td><td>— (campione solido)</td><td>biomolecole, polimeri, imaging</td></tr>
</table>
```

```html
<div class="duo"><div class="box lr"><b>A risoluzione unitaria (analizzatori quadrupolari convenzionali)</b>
<p>Lo spettro EI dà masse nominali: 43 può essere C<sub>3</sub>H<sub>7</sub><sup>+</sup> o C<sub>2</sub>H<sub>3</sub>O<sup>+</sup>. La formula si deduce da isotopi, regole e contesto (capitoli 13 e 15).</p></div>
<div class="box hr"><b>Ad alta risoluzione (GC-TOF, GC-Orbitrap)</b>
<p>Ogni frammento EI ha la sua massa esatta: 43,0542 (C<sub>3</sub>H<sub>7</sub><sup>+</sup>) e 43,0178 (C<sub>2</sub>H<sub>3</sub>O<sup>+</sup>) sono picchi diversi. Le formule dei frammenti si verificano in ppm (capitolo 11), il che rende molto più sicura l'interpretazione.</p></div></div>
```

```html
<div class="box oral"><b>Concetti chiave e domande di ripasso</b>
<p>Descrivere la geometria di una sorgente EI e il processo di ionizzazione; comprendere perché si usano 70 eV e perché gli spettri EI sono storicamente riproducibili e confrontabili in libreria; definire lo ione radicale molecolare (elettroni dispari); spiegare perché lo ione molecolare può essere debole o assente e quali strategie permettono di osservarlo (CI, EI a bassa energia); principi di funzionamento della CI e criteri di scelta del gas reagente; ionizzazione chimica negativa (NCI) per analiti alogenati ed elettrofili; confronto tra le diverse sorgenti di ionizzazione in funzione della natura chimica dell'analita.</p></div>
```

## Verifica

??? Perché in EI conta solo la frammentazione unimolecolare, mentre in CI ci sono reazioni ione-molecola?
In EI la pressione è ~10^−5^ Torr: il cammino libero medio è molto più lungo della sorgente e lo ione non incontra altre molecole. In CI il gas reagente è a ~1 Torr: cammino libero medio di decine di micrometri, migliaia di urti, e la ionizzazione stessa è una reazione ione-molecola.

```html
<details class="q"><summary>Nello spettro CI (metano) di un composto vedi m/z 151, 179 e 191. Qual è la massa molecolare?</summary>
<p>150: [M+H]<sup>+</sup> = 151, [M+C<sub>2</sub>H<sub>5</sub>]<sup>+</sup> = 179 (M+29), [M+C<sub>3</sub>H<sub>5</sub>]<sup>+</sup> = 191 (M+41).</p></details>
```

```html
<details class="q"><summary>L'analita ha PA = 800 kJ/mol. Con quale gas reagente ottieni lo spettro più semplice?</summary>
<p>Con l'isobutano: PA(isobutene) = 802 kJ/mol, la protonazione è appena endotermica o quasi termoneutra, quindi dà [M+H]<sup>+</sup> con poca energia (o l'addotto); con il metano (543) sarebbe esotermica di ~260 kJ/mol (~2,7 eV) e darebbe frammenti; con l'ammoniaca (854) non si protona e si vedrebbe al più [M+NH<sub>4</sub>]<sup>+</sup>.</p></details>
```

??? Si propone di analizzare in GC-EI un prodotto di trasformazione polare senza derivatizzazione né preparazione. Quali sono le criticità?
I prodotti polari e idrofili sono tipicamente poco volatili e termolabili (presenza di gruppi –OH, –NH, carbossilici): possono non attraversare la colonna GC, produrre forte asimmetria (code) o decomporsi termicamente nell'iniettore. Senza derivatizzazione (capitolo 5), la tecnica d'elezione per queste specie è la LC-ESI.

## Per approfondire

```html
<ul class="refs">
<li><span class="tag">libro</span>McLafferty F.W., Tureček F. <i>Interpretation of Mass Spectra</i>, 4ª ed., University Science Books, 1993: capitoli 1, 6 e 7.</li>
<li><span class="tag">libro</span>Gross J.H. <i>Mass Spectrometry: A Textbook</i>, 3ª ed., Springer, 2017: capitoli 5 (EI) e 7 (CI). <a href="https://doi.org/10.1007/978-3-319-54398-7" target="_blank" rel="noopener">doi:10.1007/978-3-319-54398-7</a></li>
<li><span class="tag">articolo</span>Mikaia A. (2022) Protocol for Structure Determination of Unknowns by EI Mass Spectrometry. I. <i>J. Phys. Chem. Ref. Data</i> 51, 031501. <a href="https://doi.org/10.1063/5.0091956" target="_blank" rel="noopener">doi:10.1063/5.0091956</a></li>
<li><span class="tag">articolo</span>Stein S.E. (1994) Estimating probabilities of correct identification from results of mass spectral library searches. <i>J. Am. Soc. Mass Spectrom.</i> 5, 316-323. <a href="https://doi.org/10.1016/1044-0305(94)85022-4" target="_blank" rel="noopener">doi:10.1016/1044-0305(94)85022-4</a></li>
<li><span class="tag">web</span><a href="https://webbook.nist.gov/chemistry/" target="_blank" rel="noopener">NIST Chemistry WebBook</a>: spettri EI, energie di ionizzazione e affinità protoniche (consultazione).</li>
</ul>
```
