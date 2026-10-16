"""Reference answers of tpmine IRLS logistic regression for crates/mzlab-core/tests/tpmine_irls_parity.rs."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "TP_Mine" / "py"))
from tpmine.isf import fit_logistic, predict_logit  # noqa: E402


def make_features_and_labels(
    rng: np.random.Generator,
    n: int,
    m: int,
    nan_ratio: float,
    has_weights: bool,
    is_constant_col: bool,
) -> tuple[np.ndarray, np.ndarray, np.ndarray | None]:
    """Generates synthetic features X, labels y and optional weights w."""
    x = rng.normal(loc=0.5, scale=2.0, size=(n, m))
    if nan_ratio > 0.0:
        mask = rng.uniform(size=(n, m)) < nan_ratio
        for j in range(m):
            if np.sum(~mask[:, j]) < 2:
                mask[:2, j] = False
        x[mask] = np.nan

    if is_constant_col and m >= 2:
        x[:, 0] = 3.5

    true_beta = rng.normal(scale=1.0, size=m)
    x_imputed = np.nan_to_num(x, nan=0.0)
    eta = np.clip(x_imputed @ true_beta + rng.normal(scale=0.5, size=n), -10.0, 10.0)
    p = 1.0 / (1.0 + np.exp(-eta))
    y = (rng.uniform(size=n) < p).astype(float)
    if np.all(y == 0.0):
        y[0] = 1.0
    elif np.all(y == 1.0):
        y[0] = 0.0

    w = rng.uniform(0.2, 3.0, size=n) if has_weights else None
    return x, y, w


def evaluate_queries(rng: np.random.Generator, m: int, model_dict: dict) -> list[dict]:
    """Generates and evaluates diverse query vectors for predict_logit."""
    queries = []
    q_candidates = [
        rng.normal(size=m).tolist(),
        [val if idx % 2 == 0 else float("nan") for idx, val in enumerate(rng.normal(size=m))],
        [float("nan")] * m,
        (rng.choice([-50.0, 50.0], size=m)).tolist(),
    ]
    for q_vec in q_candidates:
        feat_dict = {f"f{j}": val for j, val in enumerate(q_vec)}
        logit, contrib_dict = predict_logit(feat_dict, model_dict)
        contrib_vec = [contrib_dict[f"f{j}"] for j in range(m)]
        q_json = [None if np.isnan(v) else v for v in q_vec]
        queries.append({"feat": q_json, "logit": float(logit), "contrib": contrib_vec})
    return queries


def generate_single_case(
    rng: np.random.Generator,
    n: int,
    m: int,
    nan_ratio: float,
    has_weights: bool,
    lam: float,
    is_constant_col: bool = False,
) -> dict:
    """Generates a complete dataset case with reference model and queries."""
    x, y, w = make_features_and_labels(rng, n, m, nan_ratio, has_weights, is_constant_col)
    mu, sd, coef, intercept = fit_logistic(x, y, lam=lam, w=w, n_iter=60)

    model_dict = {
        "features": [f"f{j}" for j in range(m)],
        "mean": mu.tolist(),
        "std": sd.tolist(),
        "coef": coef.tolist(),
        "intercept": float(intercept),
    }

    queries = evaluate_queries(rng, m, model_dict)
    x_json = [[None if np.isnan(v) else float(v) for v in row] for row in x]

    return {
        "n": n,
        "m": m,
        "lam": float(lam),
        "n_iter": 60,
        "x": x_json,
        "y": y.tolist(),
        "w": w.tolist() if w is not None else None,
        "mean": mu.tolist(),
        "std": sd.tolist(),
        "coef": coef.tolist(),
        "intercept": float(intercept),
        "queries": queries,
    }


def generate_all_cases() -> list[dict]:
    """Generates at least 30 diverse test cases."""
    rng = np.random.default_rng(20261016)
    configurations = [
        # (n, m, nan_ratio, has_weights, lam, is_constant_col)
        (10, 2, 0.0, False, 1.0, False),
        (10, 3, 0.1, True, 2.0, False),
        (15, 2, 0.2, False, 0.5, False),
        (15, 4, 0.0, True, 1.5, False),
        (20, 3, 0.05, False, 0.1, False),
        (20, 5, 0.1, True, 3.0, False),
        (25, 2, 0.0, True, 0.01, False),
        (25, 4, 0.15, False, 5.0, False),
        (30, 3, 0.0, False, 2.0, True),  # Constant feature test
        (30, 6, 0.1, True, 1.0, False),
        (35, 4, 0.2, False, 4.0, False),
        (40, 2, 0.0, True, 0.5, False),
        (40, 5, 0.05, False, 10.0, False),
        (45, 7, 0.1, True, 2.0, False),
        (50, 3, 0.0, False, 0.2, False),
        (50, 6, 0.15, True, 1.0, False),
        (60, 4, 0.05, False, 2.5, False),
        (60, 8, 0.1, True, 0.5, False),
        (70, 2, 0.0, False, 1.0, False),
        (70, 5, 0.2, True, 3.0, False),
        (80, 3, 0.1, False, 0.8, False),
        (80, 7, 0.0, True, 2.0, False),
        (90, 4, 0.05, False, 1.2, False),
        (100, 2, 0.0, True, 0.1, False),
        (100, 5, 0.1, False, 2.0, False),
        (100, 8, 0.15, True, 5.0, False),
        (120, 3, 0.0, False, 1.0, False),
        (120, 6, 0.1, True, 0.3, False),
        (140, 4, 0.05, False, 2.0, False),
        (150, 2, 0.2, True, 1.5, False),
        (150, 5, 0.0, False, 0.5, False),
        (160, 7, 0.1, True, 4.0, False),
        (180, 3, 0.05, False, 2.0, False),
        (180, 6, 0.15, True, 1.0, False),
        (200, 2, 0.0, False, 0.05, False),
        (200, 4, 0.1, True, 2.0, False),
        (200, 8, 0.05, False, 1.0, False),
    ]
    cases = []
    for n, m, nan_ratio, has_weights, lam, const_col in configurations:
        case = generate_single_case(rng, n, m, nan_ratio, has_weights, lam, const_col)
        cases.append(case)
    return cases


def main() -> None:
    cases = generate_all_cases()
    data = {"cases": cases}
    content = json.dumps(data, indent=2)
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(content, encoding="utf-8")
    else:
        sys.stdout.write(content)


if __name__ == "__main__":
    main()
