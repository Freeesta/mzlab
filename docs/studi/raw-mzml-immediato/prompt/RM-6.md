# RM-6 — Banco della centroidatura (scansioni senza lista)

Prompt per una chat DEV: copia tutto ciò che segue la riga «---». Dopo RM-0; indipendente dagli altri. È anche il banco che T-7e (B4) usa per un algoritmo nuovo.

---

Pacchetto RM-6 del convertitore Thermo `.raw` → mzML di mzLab. Leggi prima `docs/studi/raw-mzml-immediato/prompt/COMUNI.md`, poi in `docs/studi/raw-mzml-immediato/` la sezione C di `CRITERI.md` e `RISPOSTE.md` §1.1. Leggi anche la riga B4 di `docs/studi/PROMPT_LETTORE_THERMO/T-7.md`.

**Leggi**:
- `crates/mzlab-core/src/centroidi/` (algoritmi di oggi);
- `crates/mzlab-core/src/lettori_centroidi.rs` e `lettori_centroidi/prove.rs`.

**Tocca**:
- un banco nuovo (esempio o test in `crates/mzlab-core`) con il simulatore e le metriche di C1-C4;
- `docs/agenti/raw-mzml.md`;
- `tools/` per i soli grafici del rapporto, se servono.

**Non tocca**: gli algoritmi di centroidatura, se non con un parametro nuovo spento di base (decisione di T-7) e la sua tabella.

## Passi
1. **Simulatore deterministico** (seme fisso). La forma di riga FT si stima dai pezzi di profilo di LAB-B: un pezzo per centroide, ~5,8 bin; si stima la larghezza in funzione di m/z. Aggiunge rumore gaussiano + poissoniano, coppie a 0,5-2 FWHM e S/N da 3 a 10⁴. Le verità sono scritte accanto.
2. **Metriche C1**: Δm/z (ppm e frazione di FWHM), area, risoluzione delle coppie, fantasmi e richiamo, per ogni algoritmo di `centroidi/` con i parametri di oggi.
3. **C2**: si centroida il profilo delle scansioni FT di LAB-B e si confronta con la lista dello strumento (precisione e richiamo entro 1 ppm, rapporto d'intensità).
4. **C3**: timololo (317,16419; ³⁴S 319,15998; ¹³C 318,16754) in LAB-B, e i calibranti di R-a quando arrivano.
5. **C4**: scansioni IT solo profilo (small.RAW, IT-HCD-SPS) contro msconvert `peakPicking vendor` (R-c).

## Accettazione
Tabella C1-C4 con tutti i numeri, anche quelli che falliscono; per ogni fallimento la causa e una proposta (un parametro nuovo spento di base). Nessun byte dell'uscita di base cambia (F10).

## Prove
Il simulatore rilegge le sue verità con un errore di 0 (test); due giri danno numeri identici.
