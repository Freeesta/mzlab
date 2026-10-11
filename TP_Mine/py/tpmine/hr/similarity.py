# TPMINE-PRIVATE
"""Modified cosine between MS2 spectra, in batches (WP8): all pairs of a few hundred candidates, or one reference against all of them, without a
Python loop over pairs or peaks. The matching is the one of `ionfamily.ms2_similarity`: a peak of one spectrum pairs with a peak of the other at the
same m/z or shifted by the difference of the precursors, heaviest pairs first, each peak used once (`ionfamily.greedy_match`, locally dominant
rounds = global greedy). numpy only.

Compare HCD with HCD of the same series. The CID spectrum of the direct infusion is a reference for formulas and genealogy, never for this
similarity (measured: the same parent gives a cosine of 0.52 between HCD and CID).

Spectral entropy similarity (Li & Fiehn, Nat. Methods 2023, 2021): intensities as probabilities, spectra of low entropy flattened by the weight
transform, score = 1 - (2 S_AB - S_A - S_B) / ln 4 with S_AB the entropy of the merged spectrum; unmatched peaks add nothing, so the sum runs
over the matched pairs only. Same peaks and matching as the modified cosine; `hybrid=True` also pairs peaks shifted by the precursor difference."""
from __future__ import annotations

import numpy as np

from mzlab import ionfamily

K = 32                      # peaks kept per spectrum
TOL_HR = 0.005              # Da, high resolution (0.5 at unit resolution)
PAD = -1e9
TYPES = ("equal", "shifted", "loss")
LOSS_MAX = 65.0             # Da: a shifted pair whose fragment lies at most this far (and at most LOSS_FRACTION of the precursor) below the precursor is a neutral loss
LOSS_FRACTION = 0.35


def prepare(mz, inten, precursor: float, k: int = K, min_rel: float = 0.01, mz_cut: float = 1.5):
    """Peaks of a spectrum ready for comparison: the precursor and everything above `precursor - mz_cut` removed, peaks below `min_rel` of the
    strongest left out, the `k` strongest kept, sqrt of the intensities, unit L2 norm. Returns (mz sorted ascending, weights)."""
    mz, inten = np.asarray(mz, float), np.asarray(inten, float)
    keep = (mz < precursor - mz_cut) & (inten > 0)
    mz, inten = mz[keep], inten[keep]
    if len(mz) == 0:
        return mz, inten
    keep = inten >= min_rel * inten.max()
    mz, inten = mz[keep], inten[keep]
    o = np.argsort(-inten, kind="stable")[:k]
    mz, w = mz[o], np.sqrt(inten[o])
    w = w / np.linalg.norm(w)
    s = np.argsort(mz, kind="stable")
    return mz[s], w[s]


def pad(spectra, precursors, k: int = K):
    """Matrices M (N, k) of m/z (padded with a huge negative number: it never matches) and W (N, k) of weights, and the precursors P (N,)."""
    n = len(spectra)
    M = np.full((n, k), PAD)
    W = np.zeros((n, k))
    for i, (m, w) in enumerate(spectra):
        M[i, :len(m)] = m[:k]
        W[i, :len(w)] = w[:k]
    return M, W, np.asarray(precursors, float)


def modified_cosine(M, W, P, ia, ib, tol: float = TOL_HR, batch: int = 1000):
    """Modified cosine and number of matched peaks of the pairs (ia[j], ib[j]). Matrices from `pad`. Pairs are processed in batches of `batch`
    (memory ~ batch x k x k x 8 bytes x 4)."""
    ia, ib = np.asarray(ia, int), np.asarray(ib, int)
    score = np.zeros(len(ia))
    nmat = np.zeros(len(ia), int)
    for s in range(0, len(ia), batch):
        a, b = ia[s:s + batch], ib[s:s + batch]
        d = (P[b] - P[a])[:, None, None]
        diff = M[b][:, None, :] - M[a][:, :, None]
        S = np.where((np.abs(diff) <= tol) | (np.abs(diff - d) <= tol), W[a][:, :, None] * W[b][:, None, :], 0.0)
        score[s:s + batch], nmat[s:s + batch] = ionfamily.greedy_match(S)
    return score, nmat


def entropy_weights(W, low: float = 3.0):
    """Intensity weights of the entropy similarity, rows of W from `pad` (W is sqrt intensity, so the intensities are W^2). Each row becomes a
    probability vector p (sum 1); when its entropy S is below `low`, p is flattened as p^(0.25 + 0.25 S) and renormalised (Li & Fiehn 2021)."""
    p = np.asarray(W, float) ** 2
    tot = p.sum(1, keepdims=True)
    p = np.divide(p, tot, out=np.zeros_like(p), where=tot > 0)
    S = -(p * np.log(np.where(p > 0, p, 1.0))).sum(1, keepdims=True)
    q = np.where(p > 0, p, 0.0) ** np.where(S < low, 0.25 + 0.25 * S, 1.0)
    tot = q.sum(1, keepdims=True)
    return np.divide(q, tot, out=np.zeros_like(q), where=tot > 0)


def _xlogx(x):
    return np.where(x > 0, x * np.log(np.where(x > 0, x, 1.0)), 0.0)


def entropy_similarity(M, W, P, ia, ib, tol: float = TOL_HR, hybrid: bool = True, batch: int = 1000):
    """Spectral entropy similarity (0..1) and number of matched peaks of the pairs (ia[j], ib[j]). A pair of matched peaks with probabilities
    a and b adds (a+b) ln(a+b) - a ln a - b ln b; the sum over the one-to-one matching (heaviest first, as the modified cosine) is divided by
    2 ln 2. `hybrid`: peaks at the same m/z or shifted by the difference of the precursors (the pair parent-product); otherwise same m/z only."""
    ia, ib = np.asarray(ia, int), np.asarray(ib, int)
    Q = entropy_weights(W)
    score = np.zeros(len(ia))
    nmat = np.zeros(len(ia), int)
    for s in range(0, len(ia), batch):
        a, b = ia[s:s + batch], ib[s:s + batch]
        diff = M[b][:, None, :] - M[a][:, :, None]
        ok = np.abs(diff) <= tol
        if hybrid:
            ok |= np.abs(diff - (P[b] - P[a])[:, None, None]) <= tol
        qa, qb = Q[a][:, :, None], Q[b][:, None, :]
        f = _xlogx(qa + qb) - _xlogx(qa) - _xlogx(qb)
        t, n = ionfamily.greedy_match(np.where(ok & (qa > 0) & (qb > 0), f, 0.0))
        score[s:s + batch], nmat[s:s + batch] = np.minimum(t / (2 * np.log(2)), 1.0), n
    return score, nmat


def entropy_one_against_all(M, W, P, ref: int, others=None, tol: float = TOL_HR, hybrid: bool = True, batch: int = 1000):
    """Entropy similarity of spectrum `ref` against `others` (default: all, itself excluded). Returns (others, score, n_matched)."""
    others = np.array([i for i in range(len(M)) if i != ref]) if others is None else np.asarray(others, int)
    sc, nm = entropy_similarity(M, W, P, np.full(len(others), ref), others, tol, hybrid, batch)
    return others, sc, nm


def _typed_matching(mz_a, w_a, mz_b, w_b, delta: float, prec_a: float, tol: float, loss_max: float, weight):
    """One-to-one matching (heaviest first) of the peaks of A with those of B at the same m/z or shifted by `delta`; `weight(wa, wb)` is the
    contribution of a pair. Returns a list of (i, j, type, contribution); type 'equal' (same m/z), 'shifted' (moved with the precursor: contains the
    modification) or 'loss' (moved with the precursor and lost from the precursor in A by at most min(`loss_max`, 35 % of the precursor): a neutral loss both spectra share)."""
    diff = mz_b[None, :] - mz_a[:, None]
    eq = np.abs(diff) <= tol
    ok = eq | (np.abs(diff - delta) <= tol)
    wt = np.where(ok, weight(w_a[:, None], w_b[None, :]), 0.0)
    ia, ib = np.nonzero(wt > 0)
    pick = ionfamily._greedy_pairs(ia, ib, wt[ia, ib], len(mz_a), len(mz_b))
    out = []
    for k in pick.tolist():
        i, j = int(ia[k]), int(ib[k])
        kind = "equal" if eq[i, j] else ("loss" if prec_a - mz_a[i] <= min(loss_max, LOSS_FRACTION * prec_a) else "shifted")
        out.append((i, j, kind, float(wt[i, j])))
    return out


def pair_components(spec_a, prec_a: float, spec_b, prec_b: float, tol: float = TOL_HR, loss_max: float = LOSS_MAX) -> dict:
    """Similarity of the MS2 of a parent (A) and of a product (B) in three components shown apart (Schollée et al., JASMS 2017: fragments that moved
    with the mass difference). `spec_*` = (mz, weights) from `prepare`. The peaks are paired once, as in the modified cosine, and every pair has a
    type: equal fragments (the modification is outside them), shifted fragments (they carry it) and common neutral losses (shifted pairs whose loss
    from the precursor is small, see `_typed_matching`). For each type the share of the modified cosine and of the spectral entropy similarity (hybrid, same
    typing); the three shares add up to the totals. Returns {delta, cos{equal, shifted, loss, total}, ent{...}, n{...}, pairs[{a, b, type, w}]},
    pairs of the cosine matching, for the mirror."""
    (ma, wa), (mb, wb) = (np.asarray(spec_a[0], float), np.asarray(spec_a[1], float)), (np.asarray(spec_b[0], float), np.asarray(spec_b[1], float))
    delta = float(prec_b - prec_a)
    res = {"delta": delta, "cos": dict.fromkeys(TYPES, 0.0), "ent": dict.fromkeys(TYPES, 0.0), "n": dict.fromkeys(TYPES, 0), "pairs": []}
    if len(ma) and len(mb):
        for i, j, kind, v in _typed_matching(ma, wa, mb, wb, delta, prec_a, tol, loss_max, lambda x, y: x * y):
            res["cos"][kind] += v
            res["n"][kind] += 1
            res["pairs"].append({"a": float(ma[i]), "b": float(mb[j]), "type": kind, "w": v})
        qa, qb = entropy_weights(wa[None, :])[0], entropy_weights(wb[None, :])[0]
        ent = lambda x, y: np.where((x > 0) & (y > 0), _xlogx(x + y) - _xlogx(x) - _xlogx(y), 0.0) / (2 * np.log(2))
        for _, _, kind, v in _typed_matching(ma, qa, mb, qb, delta, prec_a, tol, loss_max, ent):
            res["ent"][kind] += v
    for key in ("cos", "ent"):
        res[key]["total"] = min(1.0, sum(res[key][k] for k in TYPES))
    return res


def plain_cosine(M, W, ia, ib, tol: float = TOL_HR, batch: int = 1000):
    """Cosine with no precursor shift (peaks at the same m/z only), same matching."""
    ia, ib = np.asarray(ia, int), np.asarray(ib, int)
    score = np.zeros(len(ia))
    for s in range(0, len(ia), batch):
        a, b = ia[s:s + batch], ib[s:s + batch]
        diff = M[b][:, None, :] - M[a][:, :, None]
        S = np.where(np.abs(diff) <= tol, W[a][:, :, None] * W[b][:, None, :], 0.0)
        score[s:s + batch] = ionfamily.greedy_match(S)[0]
    return score


def bin_matrices(M, W, P, binw: float = 1.0, mmax: float = 1500.0):
    """Dense matrices F (fragments) and L (neutral losses = precursor - fragment) on `binw` bins, float32, each peak 'smeared' by +-1 bin so that a
    pair of peaks on either side of a bin edge still meets. Returns (F, L, Fs, Ls): products F @ Fs.T + L @ Ls.T bound the modified cosine."""
    n, k = M.shape
    nb = int(mmax / binw) + 2
    F = np.zeros((n, nb), np.float32)
    L = np.zeros((n, nb), np.float32)
    rows = np.repeat(np.arange(n), k)
    ok = M.ravel() > 0
    fb = np.clip((M.ravel() / binw + 0.5).astype(int), 0, nb - 1)
    lb = np.clip(((P[rows] - M.ravel()) / binw + 0.5).astype(int), 0, nb - 1)
    w = W.ravel().astype(np.float32)
    np.maximum.at(F, (rows[ok], fb[ok]), w[ok])
    np.maximum.at(L, (rows[ok], lb[ok]), w[ok])

    def smear(X):
        return np.maximum(X, np.maximum(np.roll(X, 1, 1), np.roll(X, -1, 1)))
    return F, L, smear(F), smear(L)


def upper_bound(F, L, Fs, Ls, rows=None, cols=None):
    """Prefilter score (two matrix products): >= 0.3 loses no pair whose modified cosine is >= 0.5 (measured on two real files). rows / cols:
    index arrays to compare a subset (reference spectra against candidates) instead of everything against everything."""
    r = np.arange(len(F)) if rows is None else np.asarray(rows)
    c = np.arange(len(F)) if cols is None else np.asarray(cols)
    return F[r] @ Fs[c].T + L[r] @ Ls[c].T


def pairs_above(U: np.ndarray, threshold: float = 0.3, upper_triangle: bool = True):
    """Index pairs (i, j) with U >= threshold (i < j for a square all-against-all matrix)."""
    ok = U >= threshold
    if upper_triangle:
        ok = np.triu(ok, 1)
    return np.nonzero(ok)


def one_against_all(M, W, P, ref: int, others=None, tol: float = TOL_HR, batch: int = 1000):
    """Modified cosine and matched peaks of spectrum `ref` against `others` (default: all, itself excluded). Returns (others, score, n_matched)."""
    others = np.array([i for i in range(len(M)) if i != ref]) if others is None else np.asarray(others, int)
    sc, nm = modified_cosine(M, W, P, np.full(len(others), ref), others, tol, batch)
    return others, sc, nm


def all_pairs(M, W, P, tol: float = TOL_HR, threshold: float = 0.3, batch: int = 1000, binw: float = 1.0):
    """All pairs of N spectra: prefilter with the two matrix products, exact modified cosine only where the bound reaches `threshold`.
    Returns (ia, ib, score, n_matched) of the pairs computed (i < j); pairs left out have a modified cosine below the bound's threshold."""
    F, L, Fs, Ls = bin_matrices(M, W, P, binw)
    ia, ib = pairs_above(upper_bound(F, L, Fs, Ls), threshold)
    sc, nm = modified_cosine(M, W, P, ia, ib, tol, batch)
    return ia, ib, sc, nm


def neighbours(ia, ib, score, n_matched, n: int, min_cos: float = 0.6, min_peaks: int = 4, max_neighbours: int = 10):
    """Edges of the network: modified cosine >= `min_cos` with at least `min_peaks` matched peaks, at most `max_neighbours` per node (the best).
    Returns a list of (i, j, score, n_matched) with i < j."""
    ok = (score >= min_cos) & (n_matched >= min_peaks)
    a, b, s, m = ia[ok], ib[ok], score[ok], n_matched[ok]
    if len(a) == 0:
        return []
    o = np.argsort(-s, kind="stable")
    a, b, s, m = a[o], b[o], s[o], m[o]
    count = np.zeros(n, int)
    out = []
    for i, j, sc, mm in zip(a.tolist(), b.tolist(), s.tolist(), m.tolist()):          # <= a few thousand edges
        if count[i] < max_neighbours and count[j] < max_neighbours:
            count[i] += 1
            count[j] += 1
            out.append((i, j, sc, mm))
    return out
