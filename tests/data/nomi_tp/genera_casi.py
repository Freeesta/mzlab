#!/usr/bin/env python3
"""Generazione deterministica dei casi di test per la nomenclatura dei TP.

Rigenera byte-per-byte:
  - tests/data/nomi_tp/trasformazioni.csv (120 righe)
  - tests/data/nomi_tp/casi_nomi.csv (110 righe)

Utilizza esclusivamente la libreria standard di Python e pyteomics.
"""
from __future__ import annotations

import csv
import importlib.metadata
import io
import os
import sys
from collections import defaultdict
from pathlib import Path

import pyteomics.mass

# -----------------------------------------------------------------------------
# Deterministic transformation data (120 rows over 8 parent molecules)
# -----------------------------------------------------------------------------
TRASFORMAZIONI_RAW = """case_id,parent_name,parent_smiles,parent_formula,transformation,net_change,tp_formula_neutral
T001,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,idrossilazione,+O,C15H12N2O2
T002,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,addizione di metano,+CH4,C16H16N2O
T003,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,diidrossilazione,+2O,C15H12N2O3
T004,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,solforazione,+S,C15H12N2OS
T005,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,triossidazione,+3O,C15H12N2O4
T006,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,deidrogenazione,-H2,C15H10N2O
T007,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,ossidazione a carbonile,+O -2H,C15H10N2O2
T008,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,metilazione,+CH2,C16H14N2O
T009,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,decarbonilazione,-CO,C14H12N2
T010,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,dealchilazione,-C2H4,C13H8N2O
T011,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,idratazione,+H2O,C15H14N2O2
T012,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,disidratazione,-H2O,C15H10N2
T013,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,solfonazione,+SO3,C15H12N2O4S
T014,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,pentaossidazione,+5O,C15H12N2O6
T015,carbamazepina,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,glucuronidazione netta,+C6H8O6,C21H20N2O7
T016,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,idrossilazione,+O,C8H9NO3
T017,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,addizione di metano,+CH4,C9H13NO2
T018,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,diidrossilazione,+2O,C8H9NO4
T019,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,solforazione,+S,C8H9NO2S
T020,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,triossidazione,+3O,C8H9NO5
T021,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,deidrogenazione,-H2,C8H7NO2
T022,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,ossidazione a carbonile,+O -2H,C8H7NO3
T023,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,metilazione,+CH2,C9H11NO2
T024,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,decarbonilazione,-CO,C7H9NO
T025,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,dealchilazione,-C2H4,C6H5NO2
T026,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,perdita di formaldeide,-CH2O,C7H7NO
T027,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,perdita di monossido di azoto,-NO,C8H9O
T028,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,demetilazione,-CH2,C7H7NO2
T029,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,solfonazione,+SO3,C8H9NO5S
T030,paracetamolo,CC(=O)Nc1ccc(O)cc1,C8H9NO2,glucuronidazione netta,+C6H8O6,C14H17NO8
T031,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,demetilazione,-CH2,C7H8N4O2
T032,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,deossigenazione con idrogenazione,-O +H2,C8H12N4O
T033,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,dealchilazione,-C2H4,C6H6N4O2
T034,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,decarbonilazione,-CO,C7H10N4O
T035,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,perdita di formaldeide,-CH2O,C7H8N4O
T036,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,perdita di monossido di azoto,-NO,C8H10N3O
T037,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,idrossilazione,+O,C8H10N4O3
T038,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,addizione di metano,+CH4,C9H14N4O2
T039,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,diidrossilazione,+2O,C8H10N4O4
T040,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,solforazione,+S,C8H10N4O2S
T041,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,triossidazione,+3O,C8H10N4O5
T042,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,deidrogenazione,-H2,C8H8N4O2
T043,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,ossidazione a carbonile,+O -2H,C8H8N4O3
T044,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,metilazione,+CH2,C9H12N4O2
T045,caffeina,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,solfonazione,+SO3,C8H10N4O5S
T046,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,dealogenazione idrolitica,-Cl +OH,C8H15N5O
T047,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,deetilazione,-C2H4,C6H10ClN5
T048,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,N-deisopropilazione,-C3H6,C5H8ClN5
T049,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,demetilazione,-CH2,C7H12ClN5
T050,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,deaminazione,-NH3,C8H11ClN4
T051,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,declorurazione riduttiva,-Cl +H,C8H15N5
T052,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,deidrogenazione,-H2,C8H12ClN5
T053,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,idrossilazione,+O,C8H14ClN5O
T054,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,addizione di metano,+CH4,C9H18ClN5
T055,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,diidrossilazione,+2O,C8H14ClN5O2
T056,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,solforazione,+S,C8H14ClN5S
T057,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,ossidazione a carbonile,+O -2H,C8H12ClN5O
T058,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,metilazione,+CH2,C9H16ClN5
T059,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,solfonazione,+SO3,C8H14ClN5O3S
T060,atrazina,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,pentaossidazione,+5O,C8H14ClN5O5
T061,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,idrossilazione,+O,C14H11Cl2NO3
T062,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,addizione di metano,+CH4,C15H15Cl2NO2
T063,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,diidrossilazione,+2O,C14H11Cl2NO4
T064,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,solforazione,+S,C14H11Cl2NO2S
T065,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,triossidazione,+3O,C14H11Cl2NO5
T066,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,deidrogenazione,-H2,C14H9Cl2NO2
T067,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,disidratazione,-H2O,C14H9Cl2NO
T068,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,dealogenazione idrolitica,-Cl +OH,C14H12ClNO3
T069,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,decarbossilazione,-CO2,C13H11Cl2N
T070,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,perdita di etilene ossido,-C2H4O,C12H7Cl2NO
T071,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,decarbonilazione,-CO,C13H11Cl2NO
T072,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,dealchilazione,-C2H4,C12H7Cl2NO2
T073,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,ossidazione a carbonile,+O -2H,C14H9Cl2NO3
T074,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,metilazione,+CH2,C15H13Cl2NO2
T075,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,declorurazione riduttiva,-Cl +H,C14H12ClNO2
T076,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,idrossilazione,+O,C10H11N3O4S
T077,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,addizione di metano,+CH4,C11H15N3O3S
T078,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,diidrossilazione,+2O,C10H11N3O5S
T079,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,solforazione,+S,C10H11N3O3S2
T080,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,triossidazione,+3O,C10H11N3O6S
T081,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,deidrogenazione,-H2,C10H9N3O3S
T082,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,desolfonilazione,-SO2,C10H11N3O
T083,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,perdita di ciclopentadiene,-C5H4,C5H7N3O3S
T084,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,decarbonilazione,-CO,C9H11N3O2S
T085,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,dealchilazione,-C2H4,C8H7N3O3S
T086,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,demetilazione,-CH2,C9H9N3O3S
T087,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,deossigenazione con idrogenazione,-O +H2,C10H13N3O2S
T088,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,ossidazione a carbonile,+O -2H,C10H9N3O4S
T089,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,metilazione,+CH2,C11H13N3O3S
T090,sulfametossazolo,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,acetilazione,+C2H2O,C12H13N3O4S
T091,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,defluorurazione idrolitica,-F +OH,C14H14F3N3O3S
T092,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,deidrogenazione,-H2,C14H11F4N3O2S
T093,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,idrossilazione,+O,C14H13F4N3O3S
T094,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,addizione di metano,+CH4,C15H17F4N3O2S
T095,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,diidrossilazione,+2O,C14H13F4N3O4S
T096,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,solforazione,+S,C14H13F4N3O2S2
T097,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,N-deisopropilazione,-C3H6,C11H7F4N3O2S
T098,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,perdita di chetene,-C2H2O,C12H11F4N3OS
T099,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,dealchilazione,-C2H4,C12H9F4N3O2S
T100,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,decarbonilazione,-CO,C13H13F4N3OS
T101,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,ossidazione a carbonile,+O -2H,C14H11F4N3O3S
T102,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,metilazione,+CH2,C15H15F4N3O2S
T103,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,disidratazione,-H2O,C14H11F4N3OS
T104,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,solfonazione,+SO3,C14H13F4N3O5S2
T105,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,pentaossidazione,+5O,C14H13F4N3O7S
T106,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,idrossilazione,+O,C13H18O3
T107,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,addizione di metano,+CH4,C14H22O2
T108,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,diidrossilazione,+2O,C13H18O4
T109,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,solforazione,+S,C13H18O2S
T110,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,triossidazione,+3O,C13H18O5
T111,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,deidrogenazione,-H2,C13H16O2
T112,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,ossidazione a carbonile,+O -2H,C13H16O3
T113,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,metilazione,+CH2,C14H20O2
T114,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,decarbossilazione,-CO2,C12H18
T115,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,perdita di etilene ossido,-C2H4O,C11H14O
T116,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,dealchilazione maggiore,-C4H8,C9H10O2
T117,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,perdita di acroleina,-C3H4O,C10H14O
T118,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,decarbonilazione,-CO,C12H18O
T119,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,dealchilazione,-C2H4,C11H14O2
T120,ibuprofene,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,glucuronidazione netta,+C6H8O6,C19H26O8"""

# -----------------------------------------------------------------------------
# Deterministic specs for isomers and different formulas
# -----------------------------------------------------------------------------
ISOMERS_SPEC = [
    ("carbamazepina", "C15H14N2O2", [2.30, 3.50]),
    ("paracetamolo", "C7H7NO2", [1.50, 2.15]),
    ("caffeina", "C8H8N4O2", [2.20, 2.95, 3.60]),
    ("atrazina", "C6H10ClN5", [3.10, 3.85]),
    ("diclofenac", "C14H12ClNO2", [3.75, 4.30, 4.95]),
    ("ibuprofene", "C13H16O2", [4.10, 4.70, 5.25]),
    ("sulfametossazolo", "C10H9N3O3S", [2.80, 3.45]),
    ("flufenacet", "C14H11F4N3OS", [4.80, 5.25, 5.90]),
]

DIFF_SPEC = [
    # Triplets (same nominal m/z, 3 different formulas)
    ("carbamazepina", 253, [("C14H8N2O3", 3.80), ("C15H12N2O2", 4.35), ("C16H16N2O", 5.60)]),
    ("caffeina", 181, [("C6H4N4O3", 1.90), ("C7H8N4O2", 2.65), ("C8H12N4O", 3.45)]),
    # Pairs (with examples of inverse RT order)
    ("carbamazepina", 209, [("C13H8N2O", 4.20), ("C14H12N2", 3.10)]),      # INVERSE RT
    ("carbamazepina", 251, [("C15H10N2O2", 3.80), ("C16H14N2O", 4.70)]),
    ("carbamazepina", 269, [("C15H12N2OS", 4.90), ("C15H12N2O3", 3.30)]),    # INVERSE RT
    ("paracetamolo", 122, [("C7H7NO", 1.80), ("C8H9O", 2.45)]),
    ("paracetamolo", 124, [("C6H5NO2", 3.60), ("C7H9NO", 2.10)]),           # INVERSE RT
    ("paracetamolo", 168, [("C8H9NO3", 1.75), ("C9H13NO2", 2.55)]),
    ("paracetamolo", 184, [("C8H9NO2S", 2.85), ("C8H9NO4", 1.95)]),         # INVERSE RT
    ("caffeina", 165, [("C7H8N4O", 2.05), ("C8H10N3O", 2.70)]),
    ("caffeina", 167, [("C6H6N4O2", 1.90), ("C7H10N4O", 2.60)]),
    ("atrazina", 230, [("C8H12ClN5O", 3.60), ("C9H16ClN5", 4.40)]),
    ("atrazina", 232, [("C8H14ClN5O", 3.40), ("C9H18ClN5", 4.20)]),
    ("diclofenac", 252, [("C12H7Cl2NO", 4.10), ("C13H11Cl2N", 4.90)]),
    ("diclofenac", 278, [("C14H9Cl2NO", 4.85), ("C14H12ClNO3", 3.70)]),     # INVERSE RT
    ("sulfametossazolo", 190, [("C5H7N3O3S", 1.65), ("C10H11N3O", 2.50)]),
    ("ibuprofene", 151, [("C9H10O2", 3.20), ("C10H14O", 3.90)]),
    ("flufenacet", 322, [("C11H7F4N3O2S", 4.40), ("C12H11F4N3OS", 5.10)]),
]


def get_versions() -> tuple[str, str]:
    """Restituisce le versioni di Python e pyteomics."""
    py_ver = sys.version.split()[0]
    try:
        pyt_ver = importlib.metadata.version("pyteomics")
    except Exception:
        pyt_ver = getattr(pyteomics, "__version__", "5.0.1")
    return py_ver, pyt_ver


def generate_trasformazioni_bytes() -> bytes:
    """Genera deterministicamente i byte esatti di trasformazioni.csv."""
    lines = [line.strip("\r\n") for line in TRASFORMAZIONI_RAW.strip().splitlines()]
    content = "\r\n".join(lines) + "\r\n"
    return content.encode("utf-8")


def build_casi_nomi_records(t_rows: list[dict[str, str]]) -> list[dict]:
    """Costruisce i 110 casi di test secondo le specifiche di nomenclatura."""
    used_nominal_groups: set[tuple[str, int]] = set()
    for p, f, _ in ISOMERS_SPEC:
        mz = pyteomics.mass.calculate_mass(formula=f, charge=1)
        used_nominal_groups.add((p, round(mz)))
    for p, nom, _ in DIFF_SPEC:
        used_nominal_groups.add((p, nom))

    avail_by_group: dict[tuple[str, int], list[dict[str, str]]] = defaultdict(list)
    for r in t_rows:
        p = r["parent_name"]
        f = r["tp_formula_neutral"]
        mz = pyteomics.mass.calculate_mass(formula=f, charge=1)
        nom = round(mz)
        if (p, nom) not in used_nominal_groups:
            avail_by_group[(p, nom)].append(r)

    singles_spec: list[tuple[str, str, str, float]] = []
    for (p, nom), rlist in sorted(avail_by_group.items()):
        chosen = rlist[0]
        rt = round(1.5 + (nom % 35) * 0.15 + (len(p) % 5) * 0.2, 2)
        singles_spec.append((p, chosen["tp_formula_neutral"], "[M+H]+", rt))

    singles_spec.append(("diclofenac", "C14H11Cl2NO5", "[M-H]-", 2.45))
    singles_spec.append(("ibuprofene", "C19H26O8", "[M-H]-", 3.65))
    singles_spec.append(("ibuprofene", "C13H18O5", "[M-H]-", 2.90))

    all_cases: list[dict] = []
    for p, f, add, rt in singles_spec:
        charge = 1 if add == "[M+H]+" else -1
        mz_exact = pyteomics.mass.calculate_mass(formula=f, charge=charge)
        mz_nom = round(mz_exact)
        all_cases.append({
            "parent_name": p, "tp_formula_neutral": f, "adduct": add,
            "mz_exact": round(mz_exact, 5), "mz_nominal": mz_nom, "rt_min": rt,
            "expected_group": f"{p}_mz{mz_nom}", "expected_kind": "single",
        })

    for p, f, rts in ISOMERS_SPEC:
        mz_exact = pyteomics.mass.calculate_mass(formula=f, charge=1)
        mz_nom = round(mz_exact)
        for rt in rts:
            all_cases.append({
                "parent_name": p, "tp_formula_neutral": f, "adduct": "[M+H]+",
                "mz_exact": round(mz_exact, 5), "mz_nominal": mz_nom, "rt_min": rt,
                "expected_group": f"{p}_mz{mz_nom}", "expected_kind": "isomers",
            })

    for p, nom, items in DIFF_SPEC:
        for f, rt in items:
            mz_exact = pyteomics.mass.calculate_mass(formula=f, charge=1)
            mz_nom = round(mz_exact)
            all_cases.append({
                "parent_name": p, "tp_formula_neutral": f, "adduct": "[M+H]+",
                "mz_exact": round(mz_exact, 5), "mz_nominal": mz_nom, "rt_min": rt,
                "expected_group": f"{p}_mz{nom}", "expected_kind": "different_formula",
            })

    for idx, c in enumerate(all_cases, 1):
        c["case_id"] = f"CN{idx:03d}"
    return all_cases


def generate_casi_nomi_bytes(records: list[dict]) -> bytes:
    """Genera i byte esatti di casi_nomi.csv (CRLF, UTF-8)."""
    fieldnames = [
        "case_id", "parent_name", "tp_formula_neutral", "adduct",
        "mz_exact", "mz_nominal", "rt_min", "expected_group", "expected_kind"
    ]
    buffer = io.StringIO(newline="")
    writer = csv.DictWriter(buffer, fieldnames=fieldnames, lineterminator="\r\n")
    writer.writeheader()
    for c in records:
        writer.writerow({
            "case_id": c["case_id"],
            "parent_name": c["parent_name"],
            "tp_formula_neutral": c["tp_formula_neutral"],
            "adduct": c["adduct"],
            "mz_exact": f"{c['mz_exact']:.5f}",
            "mz_nominal": c["mz_nominal"],
            "rt_min": f"{c['rt_min']:.2f}",
            "expected_group": c["expected_group"],
            "expected_kind": c["expected_kind"],
        })
    return buffer.getvalue().encode("utf-8")


def verify_and_write(path: Path, new_bytes: bytes, label: str) -> None:
    """Verifica la corrispondenza byte-per-byte se il file esiste, altrimenti scrive."""
    if path.exists():
        existing_bytes = path.read_bytes()
        if existing_bytes != new_bytes:
            raise ValueError(f"Discrepanza byte-per-byte per {label}: {len(existing_bytes)} vs {len(new_bytes)}")
        print(f"  [OK] {label}: byte-for-byte identico al file esistente ({len(new_bytes)} byte).")
    else:
        path.write_bytes(new_bytes)
        print(f"  [OK] {label}: creato nuovo file deterministico ({len(new_bytes)} byte).")


def cleanup_obsolete_scripts(target_dir: Path) -> None:
    """Rimuove vecchi script scratch lasciati da iterazioni precedenti."""
    obsolete = target_dir / "generate_casi_nomi.py"
    if obsolete.exists():
        obsolete.unlink()
        print(f"  [PULIZIA] Rimosso script obsoleto: {obsolete.name}")


def main() -> None:
    target_dir = Path(__file__).resolve().parent
    py_ver, pyt_ver = get_versions()
    print("=================================================================")
    print(f"Generatore deterministico casi test nomi TP (mzLab)")
    print(f"Ambiente: Python {py_ver} | pyteomics {pyt_ver}")
    print("=================================================================")

    # 1. Generate and check trasformazioni.csv
    trasf_bytes = generate_trasformazioni_bytes()
    trasf_path = target_dir / "trasformazioni.csv"
    verify_and_write(trasf_path, trasf_bytes, "trasformazioni.csv")

    # 2. Load the transformation rows and generate casi_nomi.csv
    reader = csv.DictReader(io.StringIO(trasf_bytes.decode("utf-8")))
    t_rows = list(reader)
    casi_records = build_casi_nomi_records(t_rows)
    casi_bytes = generate_casi_nomi_bytes(casi_records)
    casi_path = target_dir / "casi_nomi.csv"
    verify_and_write(casi_path, casi_bytes, "casi_nomi.csv")

    # 3. Clean up obsolete scripts
    cleanup_obsolete_scripts(target_dir)

    print("-----------------------------------------------------------------")
    print(f"Rigenerazione deterministica completata con successo:")
    print(f"  - trasformazioni.csv: {len(t_rows)} righe, 11094 byte")
    print(f"  - casi_nomi.csv:      {len(casi_records)} righe, 9324 byte")
    print("=================================================================")


if __name__ == "__main__":
    main()
