"""A long-by-construction e2e has a cap of its own, never below the --timeout of the run."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import verifica  # noqa: E402


def test_the_sweep_has_more_than_the_ci_cap():
    assert verifica.cap_of("e2e_impaginazione", 150) > 150


def test_a_higher_timeout_wins_and_other_tests_keep_theirs():
    assert verifica.cap_of("e2e_impaginazione", 900) == 900
    assert verifica.cap_of("e2e_lingua", 150) == 150
