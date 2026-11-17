"""The e2e that need `sito` run only when named (--solo) or when notte.yml says the full site is in place (--sito-pronto)."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import verifica  # noqa: E402


def why(need, **kw):
    base = dict(dam=True, src="sintetici", only=False, sito_pronto=False)
    return verifica.skip_why(set(need), **{**base, **kw})


def test_sito_skipped_in_a_part_run_without_the_flag():
    assert why({"sito"})


def test_sito_runs_when_named_or_when_the_site_is_ready():
    assert not why({"sito"}, only=True)
    assert not why({"sito"}, sito_pronto=True)


def test_the_flag_does_not_lift_the_data_needs():
    assert why({"sito", "veri"}, sito_pronto=True)
    assert why({"sito", "dam"}, dam=False, sito_pronto=True)


def test_tests_without_sito_are_unaffected():
    assert not why(set())
    assert not why({"wasm"}, sito_pronto=True)
