# RM-9 — Numpress veloce (opzione con perdita, byte uguali al riferimento)

Prompt per una chat DEV: copia tutto ciò che segue la riga «---». Ultimo per priorità: solo dopo RM-3 e solo se il banco di RM-0 sul caso denso mostra che Numpress pesa ≥ 10 % del tempo con `--numpress`.

---

Pacchetto RM-9 del convertitore Thermo `.raw` → mzML di mzLab. Leggi prima `docs/studi/raw-mzml-immediato/prompt/COMUNI.md`, poi in `docs/studi/raw-mzml-immediato/` `VALUTAZIONE_GEMINI.md` §3 e K14-K17.

**Leggi**: `crates/mzlab-core/src/numpress.rs` (codifica di riferimento e decodifica), `crates/mzlab-core/src/mzmlwrite/codifica.rs`.

**Tocca**: un codificatore veloce accanto a quello di riferimento in `numpress.rs`, i test e `docs/agenti/raw-mzml.md`.

**Non tocca**: il valore di base (Numpress resta spento: è con perdita) e la decodifica.

## Passi
1. **Codificatore SWAR** di `put_int`. Uno scarico a ciclo (`while`, non `if`): con 9 nibble nuovi e 8 rimasti il buffer da 64 bit traboccherebbe (K15). Il nibble finale dispari riempito con 0x8, come il riferimento (K16).
2. **Predizione lineare** vettorizzabile (i residui dipendono solo dai valori) con lo stesso arrotondamento del riferimento, lo stesso punto fisso di `linear_fixed_point` (K17), e la via scalare accanto.
3. **Byte uguali al bit** al codificatore di riferimento su: 10⁶ array casuali (fuzzing), array vuoti, di 1 e 2 valori, valori grandi, negativi e NaN rifiutato con errore.

## Accettazione
- Byte identici al riferimento su tutto il fuzzing e sul corpus.
- Guadagno ≥ 10 % sul tempo totale con `--numpress` sul denso, altrimenti la PR si chiude con i numeri.
- Errore massimo dichiarato: 0,5/fp per valore, cioè per m/z il valore in ppm alla m/z più piccola e alla più grande del file.
