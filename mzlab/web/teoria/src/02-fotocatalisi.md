<!-- script: sim-misc.js -->
<!-- title: Fotocatalisi con TiO2 -->
# Fotocatalisi con TiO~2~ {kicker: Capitolo 2}

{.lead} Come un semiconduttore illuminato genera ossidanti fortissimi in acqua, quali reazioni subisce una molecola organica e che forma hanno le curve concentrazione–tempo dei prodotti.

## Il semiconduttore e la luce

Il biossido di titanio è un semiconduttore: tra la **banda di valenza** (BV), piena, e la **banda di conduzione** (BC), vuota, c'è una banda proibita (*band gap*) *E*~g~ di circa 3,2 eV per l'anatasio e 3,0 eV per il rutilo. Un fotone con energia ≥ *E*~g~ promuove un elettrone in BC e lascia una **lacuna** (h^+^) in BV. La lunghezza d'onda massima utile si ricava da

```html
<span class="eq"><i>λ</i><sub>max</sub> (nm) ≈ <span class="frac"><span>1240</span><span><i>E</i><sub>g</sub> (eV)</span></span> &nbsp;→&nbsp; anatasio: ≈ 387 nm; &nbsp;rutilo: ≈ 413 nm<span class="no">(2.1)</span></span>
```

Serve quindi radiazione UV-A: la lampada del laboratorio, o circa il 4–5% dello spettro solare. Il materiale commerciale più usato come riferimento (P25) è una miscela di anatasio e rutilo, in cui il contatto tra le due fasi aiuta a separare le cariche.

```html
<figure>
<svg viewBox="0 0 860 360" role="img" aria-label="Diagramma a bande del TiO₂: un fotone porta un elettrone dalla banda di valenza a quella di conduzione; reazioni dei radicali" aria-describedby="fd-02-0">
<defs><marker id="a2" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0,0L10,5L0,10z" fill="#24231f"/></marker>
<marker id="a2b" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0,0L10,5L0,10z" fill="#2b5c8a"/></marker>
<marker id="a2o" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0,0L10,5L0,10z" fill="#c2410c"/></marker></defs>
<style>.lb{font:13px system-ui,sans-serif;fill:#24231f}.sm{font:12px system-ui,sans-serif;fill:#6b675c}.bd{font:600 14px system-ui,sans-serif}</style>
<rect x="250" y="40" width="220" height="70" fill="#dbe6f1" stroke="#2b5c8a"/><text class="bd" x="262" y="62" fill="#2b5c8a">Banda di conduzione</text><text class="sm" x="262" y="80">≈ −0,5 V vs NHE (pH 7)</text>
<rect x="250" y="250" width="220" height="70" fill="#f6dfd3" stroke="#c2410c"/><text class="bd" x="262" y="308" fill="#c2410c">Banda di valenza</text><text class="sm" x="262" y="290">≈ +2,7 V vs NHE (pH 7)</text>
<path d="M300 112 C300 150 300 210 300 248" stroke="#999" stroke-dasharray="4 4"/><text class="sm" x="308" y="185">E<tspan dy="4" font-size="9">g</tspan><tspan dy="-4"> ≈ 3,2 eV</tspan></text>
<path d="M120 120 q15 -12 30 0 t30 0 t30 0 t30 0" stroke="#7c3aed" fill="none" stroke-width="2.2" marker-end="url(#a2)"/><text class="lb" x="118" y="108" fill="#7c3aed">hν (λ &lt; 387 nm)</text>
<path d="M400 248 V114" stroke="#24231f" stroke-width="1.8" marker-end="url(#a2)"/>
<circle cx="400" cy="92" r="9" fill="#2b5c8a"/><text x="395" y="96" fill="#fff" font-size="12" font-weight="700">e⁻</text>
<circle cx="400" cy="272" r="9" fill="#fff" stroke="#c2410c" stroke-width="2"/><text x="393" y="276" fill="#c2410c" font-size="12" font-weight="700">h⁺</text>
<path d="M420 160 C440 175 440 200 420 215" stroke="#888" fill="none" stroke-dasharray="3 3" marker-end="url(#a2)"/><text class="sm" x="440" y="192">ricombinazione (ns): perdita</text>
<path d="M472 75 H600" stroke="#2b5c8a" stroke-width="1.8" marker-end="url(#a2b)"/>
<text class="lb" x="608" y="70">O<tspan dy="4" font-size="10">2</tspan><tspan dy="-4"> + e⁻ → O</tspan><tspan dy="4" font-size="10">2</tspan><tspan dy="-10" font-size="10">•−</tspan></text>
<text class="sm" x="608" y="92">→ HO<tspan dy="4" font-size="9">2</tspan><tspan dy="-4">• → H</tspan><tspan dy="4" font-size="9">2</tspan><tspan dy="-4">O</tspan><tspan dy="4" font-size="9">2</tspan><tspan dy="-4"> → •OH</tspan></text>
<path d="M472 285 H600" stroke="#c2410c" stroke-width="1.8" marker-end="url(#a2o)"/>
<text class="lb" x="608" y="262">h⁺ + H<tspan dy="4" font-size="10">2</tspan><tspan dy="-4">O / OH⁻</tspan><tspan dy="4" font-size="10">sup</tspan><tspan dy="-4"> → •OH</tspan></text>
<text class="lb" x="608" y="284">h⁺ + R (adsorbito) → R<tspan dy="-6" font-size="10">•+</tspan></text>
<text class="lb" x="608" y="306">•OH + R → R–OH, R<tspan dy="-6" font-size="10">•</tspan><tspan dy="6"> + H</tspan><tspan dy="4" font-size="10">2</tspan><tspan dy="-4">O …</tspan></text>
<text class="sm" x="20" y="350">Valori indicativi: le posizioni delle bande si spostano di circa −59 mV per unità di pH.</text>
</svg><details class="figdesc" id="fd-02-0"><summary>Descrizione della figura</summary><p>Due riquadri: in alto la banda di conduzione (circa −0,5 V vs NHE, pH 7), in basso la banda di valenza (circa +2,7 V). Una freccia ondulata, hν con λ &lt; 387 nm, arriva da sinistra; una freccia verso l'alto porta l'elettrone e⁻ dalla valenza alla conduzione (Eg ≈ 3,2 eV); una freccia tratteggiata indica la ricombinazione. Dalla conduzione parte O2 + e⁻ → O2•⁻ → HO2• → H2O2 → •OH; dalla valenza le reazioni di h⁺ con H2O/OH⁻ e con R adsorbito.</p></details>
<figcaption><b>Figura 2.1</b> Schema a bande del TiO<sub>2</sub> illuminato. La lacuna è un ossidante molto forte: ossida acqua e ossidrili di superficie a radicale •OH, o direttamente il substrato adsorbito. L'elettrone riduce l'ossigeno disciolto; senza O<sub>2</sub> (accettore di elettroni) la ricombinazione prevale e la reazione rallenta.</figcaption>
</figure>
```

## Le specie reattive

```html
<table>
<tr><th>Specie</th><th>Origine</th><th>Carattere</th></tr>
<tr><td><b>h<sup>+</sup></b> (lacuna)</td><td>BV del TiO<sub>2</sub></td><td>Ossidante fortissimo, agisce in superficie: trasferimento di un elettrone dal substrato adsorbito (radical-catione)</td></tr>
<tr><td><b>•OH</b></td><td>h<sup>+</sup> + H<sub>2</sub>O/OH<sup>−</sup> di superficie; anche da H<sub>2</sub>O<sub>2</sub></td><td><i>E</i>° ≈ +2,7 V (pH acido); poco selettivo, costanti di reazione 10<sup>8</sup>–10<sup>10</sup> M<sup>−1</sup>s<sup>−1</sup> con la maggior parte degli organici</td></tr>
<tr><td><b>O<sub>2</sub><sup>•−</sup></b> / HO<sub>2</sub><sup>•</sup></td><td>e<sup>−</sup> + O<sub>2</sub></td><td>Ossidante/riducente blando; importante soprattutto come sorgente di H<sub>2</sub>O<sub>2</sub> e per sottrarre elettroni (meno ricombinazione)</td></tr>
<tr><td><b>e<sup>−</sup></b></td><td>BC</td><td>Riduzioni (es. alogenuri di alchile, metalli): di solito minori in presenza di O<sub>2</sub></td></tr>
</table>
```

Il TiO~2~ resta inalterato (è un catalizzatore), ma la reazione avviene **sulla superficie o vicino a essa**. Per questo contano l'adsorbimento e la carica superficiale: il punto di carica zero del TiO~2~ è intorno a pH 6–7, quindi a pH più basso la superficie è positiva e attrae anioni, a pH più alto il contrario. Un inquinante ionizzabile può degradarsi a velocità molto diverse a seconda del pH.

## Che cosa succede alla molecola

Il radicale •OH reagisce in tre modi principali, che si traducono in variazioni di massa riconoscibili nello spettro:

```html
<table>
<tr><th>Meccanismo</th><th>Esempio di esito</th><th>Δ massa nominale</th></tr>
<tr><td>Addizione a un anello aromatico o a un doppio legame</td><td>Idrossilazione (Ar–H → Ar–OH)</td><td class="n">+16</td></tr>
<tr><td>Astrazione di H da C–H (specie in α a N, O, S)</td><td>N-dealchilazione: −CH<sub>2</sub> (demetilazione), −C<sub>2</sub>H<sub>4</sub> (deetilazione), −C<sub>3</sub>H<sub>6</sub> (deisopropilazione)</td><td class="n">−14, −28, −42</td></tr>
<tr><td>Astrazione di H e ossidazione successiva</td><td>CH<sub>2</sub> → C=O (carbonile/ammide)</td><td class="n">+14</td></tr>
<tr><td>Ossidazione di eteroatomi</td><td>Solfuro → solfossido → solfone; ammina terziaria → N-ossido</td><td class="n">+16, +32</td></tr>
<tr><td>Sostituzione di un alogeno</td><td>Ar–Cl → Ar–OH (declorurazione idrossilativa)</td><td class="n">−18 (−34 +16)</td></tr>
<tr><td>Idrolisi</td><td>Ammidi, esteri, carbammati: rottura della molecola in due frammenti</td><td class="n">variabile</td></tr>
<tr><td>Decarbossilazione</td><td>R–COOH → R–H</td><td class="n">−44</td></tr>
</table>
```

Le reazioni si susseguono: un TP di prima generazione viene a sua volta attaccato (idrossilazioni multiple, dealchilazioni successive, apertura dell'anello aromatico fino ad acidi carbossilici a catena corta). Non tutte le strutture arrivano rapidamente a CO~2~: l'anello s-triazinico, per esempio, è notoriamente refrattario e la fotocatalisi delle triazine si ferma all'acido cianurico, come mostrato già negli anni '90 dal gruppo di chimica ambientale di Torino (Pelizzetti, Minero e collaboratori).

```html
<div class="box warn"><b>Errore frequente</b>
<p>Pensare che ogni Δm corrisponda a un solo meccanismo. A risoluzione unitaria −28 può essere −C<sub>2</sub>H<sub>4</sub> (28,031 Da) oppure −CO (27,995 Da): la differenza è 0,036 Da, invisibile per un quadrupolo. +16 può essere un OH aromatico, un N-ossido o un solfossido: stesso m/z, strutture e tempi di ritenzione diversi.</p></div>
```

## Cinetica

### Il parent: pseudo-primo ordine

La velocità di degradazione su un catalizzatore si descrive spesso con il modello di Langmuir–Hinshelwood, in cui la reazione avviene tra specie adsorbite:

```html
<span class="eq"><i>r</i> = −<span class="frac"><span>d<i>C</i></span><span>d<i>t</i></span></span> = <span class="frac"><span><i>k</i><sub>r</sub> <i>K</i> <i>C</i></span><span>1 + <i>K</i> <i>C</i></span></span><span class="no">(2.2)</span></span>
```

Per concentrazioni basse (*KC* ≪ 1, il caso tipico in tracce) si riduce a una cinetica di pseudo-primo ordine:

```html
<span class="eq"><i>C</i>(<i>t</i>) = <i>C</i><sub>0</sub> e<sup>−<i>k</i><sub>app</sub><i>t</i></sup> &nbsp;&nbsp;⇔&nbsp;&nbsp; ln(<i>C</i><sub>0</sub>/<i>C</i>) = <i>k</i><sub>app</sub> <i>t</i>, &nbsp;&nbsp; <i>t</i><sub>1/2</sub> = ln 2 / <i>k</i><sub>app</sub><span class="no">(2.3)</span></span>
```

Con i dati MRM del parent si costruisce il grafico ln(*C*~0~/*C*) contro *t*: se è lineare, la pendenza è *k*~app~. *k*~app~ dipende da intensità della luce, carico di catalizzatore, ossigeno, pH e matrice: non è una costante della molecola, ma del sistema.

### Gli intermedi: crescono e poi calano

Per una sequenza A → B → C con costanti di primo ordine *k*~1~ e *k*~2~, l'intermedio B segue

```html
<span class="eq">[B](<i>t</i>) = [A]<sub>0</sub> <span class="frac"><span><i>k</i><sub>1</sub></span><span><i>k</i><sub>2</sub> − <i>k</i><sub>1</sub></span></span> (e<sup>−<i>k</i><sub>1</sub><i>t</i></sup> − e<sup>−<i>k</i><sub>2</sub><i>t</i></sup>), &nbsp;&nbsp; <i>t</i><sub>max</sub> = <span class="frac"><span>ln(<i>k</i><sub>2</sub>/<i>k</i><sub>1</sub>)</span><span><i>k</i><sub>2</sub> − <i>k</i><sub>1</sub></span></span><span class="no">(2.4)</span></span>
```

Il profilo a campana è la firma di un intermedio. Un TP di seconda generazione (C, se a sua volta degradabile) parte con pendenza nulla: compare in ritardo. Nella figura potete variare le costanti e vedere come la frequenza dei campionamenti influisce sulla risoluzione temporale della cinetica.

```html
<div class="sim" id="sim-kin"><h4>Cinetica A → B → C → D</h4>
<p class="note">Concentrazioni relative ad [A]<sub>0</sub>. I punti rappresentano tempi di campionamento tipici in uno studio cinetico. Nella realtà ogni TP ha anche altre vie di formazione e scomparsa: il modello serve a capire la forma delle curve, non a fittare i dati.</p></div>
```

```html
<div class="box key"><b>Dal modello cinetico ai dati strumentali</b>
<p>Per i prodotti di trasformazione spesso non sono disponibili standard analitici puri, quindi non si misurano concentrazioni assolute dirette ma <b>aree</b> dei picchi XIC. L'area di un TP è proporzionale alla concentrazione solo a parità di risposta di ionizzazione ESI, che varia sensibilmente tra composti diversi (capitolo 6). Si può quindi confrontare l'andamento nel tempo dello stesso TP, mentre il confronto quantitativo tra TP differenti richiede cautela. Nel software la tabella delle integrazioni temporali serve proprio a tracciare queste cinetiche relative.</p></div>
```

## Aspetti operativi e pratici

```html
<ul>
<li><b>Carico di catalizzatore</b>: all'aumentare della concentrazione di TiO<sub>2</sub> aumenta la frazione di luce assorbita, ma oltre una concentrazione ottimale la torbidità della sospensione diffonde la radiazione (scattering) e la velocità di reazione satura.</li>
<li><b>Sonicazione</b>: disperde gli aggregati di particelle e rende la sospensione omogenea e riproducibile.</li>
<li><b>Tempo zero</b>: il campione t0 è stato a contatto con il fotocatalizzatore al buio. Se il precursore a t0 risulta inferiore allo standard iniziale, una frazione si è <em>adsorbita</em> sulla superficie del TiO<sub>2</sub> (e viene rimossa dalla filtrazione).</li>
<li><b>Filtrazione 0,45 µm</b>: rimuove le particelle solide di catalizzatore (che danneggerebbero colonna e sorgente) e arresta la reazione. È opportuno verificare che la membrana filtrante non adsorba il composto analizzato iniettando uno standard di controllo pre e post-filtrazione.</li>
<li><b>Controlli in bianco</b>: fotolisi diretta senza catalizzatore (per quantificare la degradazione per sola luce) e adsorbimento con TiO<sub>2</sub> al buio.</li>
</ul>
```

## Verifica

??? Perché la reazione rallenta se la soluzione viene degasata con azoto?
Manca l'O~2~ che cattura gli elettroni di banda di conduzione: le coppie e^−^/h^+^ ricombinano prima di reagire, si formano meno lacune libere e meno •OH.

??? Un TP raggiunge il massimo a 10 min e uno a 45 min. Che cosa suggerisce?
Il primo è probabilmente di prima generazione e viene degradato in fretta (*k*~2~ grande); il secondo può essere di seconda generazione o un prodotto più resistente. Con l'equazione 2.4, un *t*~max~ lungo corrisponde a *k*~2~ piccolo o a una formazione ritardata.

??? Con una lampada a 450 nm il TiO~2~ anatasio funziona?
No: 450 nm corrispondono a circa 2,76 eV, meno del band gap (3,2 eV). Servono lunghezze d'onda inferiori a circa 387 nm (o materiali modificati/drogati).

## Per approfondire

```html
<ul class="refs">
<li><span class="tag">review</span>Hoffmann M.R., Martin S.T., Choi W., Bahnemann D.W. (1995) Environmental applications of semiconductor photocatalysis. <i>Chem. Rev.</i> 95, 69–96. <a href="https://doi.org/10.1021/cr00033a004" target="_blank" rel="noopener">doi:10.1021/cr00033a004</a></li>
<li><span class="tag">review</span>Fujishima A., Zhang X., Tryk D.A. (2008) TiO<sub>2</sub> photocatalysis and related surface phenomena. <i>Surf. Sci. Rep.</i> 63, 515–582. <a href="https://doi.org/10.1016/j.surfrep.2008.10.001" target="_blank" rel="noopener">doi:10.1016/j.surfrep.2008.10.001</a></li>
<li><span class="tag">review</span>Malato S. et al. (2009) Decontamination and disinfection of water by solar photocatalysis: recent overview and trends. <i>Catal. Today</i> 147, 1–59. <a href="https://www.sciencedirect.com/science/article/abs/pii/S0920586109003344" target="_blank" rel="noopener">ScienceDirect</a></li>
<li><span class="tag">critica</span>Loeb S.K. et al. (2019) The technology horizon for photocatalytic water treatment: sunrise or sunset? <i>Environ. Sci. Technol.</i> <a href="https://doi.org/10.1021/acs.est.8b05041" target="_blank" rel="noopener">doi:10.1021/acs.est.8b05041</a></li>
<li><span class="tag">ricerca</span>Lavori del gruppo di Torino su TiO<sub>2</sub> e prodotti di trasformazione (Pelizzetti, Minero, Calza e collaboratori): <a href="https://scholar.google.com/scholar?q=Calza+Minero+TiO2+photocatalysis+transformation+products" target="_blank" rel="noopener">ricerca su Google Scholar</a></li>
<li><span class="tag">web</span><a href="https://en.wikipedia.org/wiki/Photocatalysis" target="_blank" rel="noopener">Photocatalysis</a> (Wikipedia, in inglese) per un primo inquadramento.</li>
</ul>
```
