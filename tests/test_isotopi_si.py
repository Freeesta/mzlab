"""The silicon isotopes of the envelope calculator (ionfamily) are the IUPAC abundances of the isotope score (chem.isoscore)."""

from mzlab.chem.isoscore import ISO
from mzlab.ionfamily import _ISO


def test_silicon_abundances_are_the_isotope_score_ones():
    assert [p for _, p in _ISO["Si"]] == [p for _, p in ISO["Si"]]
    assert [s for s, _ in _ISO["Si"]] == [0, 1, 2]
