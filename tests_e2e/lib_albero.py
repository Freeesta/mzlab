"""Synthetic MSn infusion for the precursor-tree panel: `big_nodes()` has 8 MS2 paths (4 precursors x 2 energies), each with one MS3 and one MS4 child (24 rows), the
numbers of scans per path being the caller's choice (5000 reproduces the order of magnitude of a real infusion). No real data."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import ROOT
sys.path.insert(0, str(ROOT))


def big_nodes():
    peaks = lambda base: [(float(base - d), 100 / (1 + i)) for i, d in enumerate((0, 18, 28, 44, 59))]
    nodes = {(): [(317.0, 100), (301.0, 3.0)]}
    for pre in (317, 301, 261, 195):
        for ce in (25, 35):
            nodes[((pre, ce),)] = peaks(pre)
            for kid in (pre - 18,):
                nodes[((pre, ce), (kid, 30))] = peaks(kid)
                nodes[((pre, ce), (kid, 30), (kid - 28, 30))] = peaks(kid - 28)
    return nodes
