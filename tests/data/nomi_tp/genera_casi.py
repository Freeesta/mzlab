#!/usr/bin/env python3
"""Deterministic generation of the test cases for TP naming.

Regenerates byte for byte:
  - tests/data/nomi_tp/trasformazioni.csv (120 rows)
  - tests/data/nomi_tp/casi_nomi.csv (110 rows)

Uses only the Python standard library and pyteomics.
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
TRANSFORMATIONS_RAW = """case_id,parent_name,parent_smiles,parent_formula,transformation,net_change,tp_formula_neutral
T001,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,hydroxylation,+O,C15H12N2O2
T002,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,methane addition,+CH4,C16H16N2O
T003,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,dihydroxylation,+2O,C15H12N2O3
T004,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,sulfation,+S,C15H12N2OS
T005,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,trioxidation,+3O,C15H12N2O4
T006,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,dehydrogenation,-H2,C15H10N2O
T007,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,oxidation to carbonyl,+O -2H,C15H10N2O2
T008,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,methylation,+CH2,C16H14N2O
T009,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,decarbonylation,-CO,C14H12N2
T010,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,dealkylation,-C2H4,C13H8N2O
T011,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,hydration,+H2O,C15H14N2O2
T012,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,dehydration,-H2O,C15H10N2
T013,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,sulfonation,+SO3,C15H12N2O4S
T014,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,pentaoxidation,+5O,C15H12N2O6
T015,carbamazepine,NC(=O)N1c2ccccc2C=Cc2ccccc21,C15H12N2O,net glucuronidation,+C6H8O6,C21H20N2O7
T016,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,hydroxylation,+O,C8H9NO3
T017,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,methane addition,+CH4,C9H13NO2
T018,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,dihydroxylation,+2O,C8H9NO4
T019,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,sulfation,+S,C8H9NO2S
T020,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,trioxidation,+3O,C8H9NO5
T021,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,dehydrogenation,-H2,C8H7NO2
T022,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,oxidation to carbonyl,+O -2H,C8H7NO3
T023,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,methylation,+CH2,C9H11NO2
T024,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,decarbonylation,-CO,C7H9NO
T025,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,dealkylation,-C2H4,C6H5NO2
T026,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,loss of formaldehyde,-CH2O,C7H7NO
T027,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,loss of nitric oxide,-NO,C8H9O
T028,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,demethylation,-CH2,C7H7NO2
T029,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,sulfonation,+SO3,C8H9NO5S
T030,paracetamol,CC(=O)Nc1ccc(O)cc1,C8H9NO2,net glucuronidation,+C6H8O6,C14H17NO8
T031,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,demethylation,-CH2,C7H8N4O2
T032,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,deoxygenation with hydrogenation,-O +H2,C8H12N4O
T033,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,dealkylation,-C2H4,C6H6N4O2
T034,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,decarbonylation,-CO,C7H10N4O
T035,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,loss of formaldehyde,-CH2O,C7H8N4O
T036,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,loss of nitric oxide,-NO,C8H10N3O
T037,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,hydroxylation,+O,C8H10N4O3
T038,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,methane addition,+CH4,C9H14N4O2
T039,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,dihydroxylation,+2O,C8H10N4O4
T040,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,sulfation,+S,C8H10N4O2S
T041,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,trioxidation,+3O,C8H10N4O5
T042,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,dehydrogenation,-H2,C8H8N4O2
T043,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,oxidation to carbonyl,+O -2H,C8H8N4O3
T044,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,methylation,+CH2,C9H12N4O2
T045,caffeine,Cn1c(=O)c2c(ncn2C)n(C)c1=O,C8H10N4O2,sulfonation,+SO3,C8H10N4O5S
T046,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,hydrolytic dehalogenation,-Cl +OH,C8H15N5O
T047,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,deethylation,-C2H4,C6H10ClN5
T048,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,N-deisopropylation,-C3H6,C5H8ClN5
T049,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,demethylation,-CH2,C7H12ClN5
T050,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,deamination,-NH3,C8H11ClN4
T051,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,reductive dechlorination,-Cl +H,C8H15N5
T052,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,dehydrogenation,-H2,C8H12ClN5
T053,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,hydroxylation,+O,C8H14ClN5O
T054,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,methane addition,+CH4,C9H18ClN5
T055,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,dihydroxylation,+2O,C8H14ClN5O2
T056,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,sulfation,+S,C8H14ClN5S
T057,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,oxidation to carbonyl,+O -2H,C8H12ClN5O
T058,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,methylation,+CH2,C9H16ClN5
T059,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,sulfonation,+SO3,C8H14ClN5O3S
T060,atrazine,CCNc1nc(Cl)nc(NC(C)C)n1,C8H14ClN5,pentaoxidation,+5O,C8H14ClN5O5
T061,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,hydroxylation,+O,C14H11Cl2NO3
T062,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,methane addition,+CH4,C15H15Cl2NO2
T063,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,dihydroxylation,+2O,C14H11Cl2NO4
T064,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,sulfation,+S,C14H11Cl2NO2S
T065,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,trioxidation,+3O,C14H11Cl2NO5
T066,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,dehydrogenation,-H2,C14H9Cl2NO2
T067,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,dehydration,-H2O,C14H9Cl2NO
T068,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,hydrolytic dehalogenation,-Cl +OH,C14H12ClNO3
T069,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,decarboxylation,-CO2,C13H11Cl2N
T070,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,loss of ethylene oxide,-C2H4O,C12H7Cl2NO
T071,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,decarbonylation,-CO,C13H11Cl2NO
T072,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,dealkylation,-C2H4,C12H7Cl2NO2
T073,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,oxidation to carbonyl,+O -2H,C14H9Cl2NO3
T074,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,methylation,+CH2,C15H13Cl2NO2
T075,diclofenac,O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl,C14H11Cl2NO2,reductive dechlorination,-Cl +H,C14H12ClNO2
T076,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,hydroxylation,+O,C10H11N3O4S
T077,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,methane addition,+CH4,C11H15N3O3S
T078,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,dihydroxylation,+2O,C10H11N3O5S
T079,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,sulfation,+S,C10H11N3O3S2
T080,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,trioxidation,+3O,C10H11N3O6S
T081,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,dehydrogenation,-H2,C10H9N3O3S
T082,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,desulfonylation,-SO2,C10H11N3O
T083,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,loss of cyclopentadiene,-C5H4,C5H7N3O3S
T084,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,decarbonylation,-CO,C9H11N3O2S
T085,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,dealkylation,-C2H4,C8H7N3O3S
T086,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,demethylation,-CH2,C9H9N3O3S
T087,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,deoxygenation with hydrogenation,-O +H2,C10H13N3O2S
T088,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,oxidation to carbonyl,+O -2H,C10H9N3O4S
T089,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,methylation,+CH2,C11H13N3O3S
T090,sulfamethoxazole,Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1,C10H11N3O3S,acetylation,+C2H2O,C12H13N3O4S
T091,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,hydrolytic defluorination,-F +OH,C14H14F3N3O3S
T092,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,dehydrogenation,-H2,C14H11F4N3O2S
T093,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,hydroxylation,+O,C14H13F4N3O3S
T094,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,methane addition,+CH4,C15H17F4N3O2S
T095,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,dihydroxylation,+2O,C14H13F4N3O4S
T096,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,sulfation,+S,C14H13F4N3O2S2
T097,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,N-deisopropylation,-C3H6,C11H7F4N3O2S
T098,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,loss of ketene,-C2H2O,C12H11F4N3OS
T099,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,dealkylation,-C2H4,C12H9F4N3O2S
T100,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,decarbonylation,-CO,C13H13F4N3OS
T101,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,oxidation to carbonyl,+O -2H,C14H11F4N3O3S
T102,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,methylation,+CH2,C15H15F4N3O2S
T103,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,dehydration,-H2O,C14H11F4N3OS
T104,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,sulfonation,+SO3,C14H13F4N3O5S2
T105,flufenacet,CC(C)N(C1=CC=C(C=C1)F)C(=O)COC2=NN=C(S2)C(F)(F)F,C14H13F4N3O2S,pentaoxidation,+5O,C14H13F4N3O7S
T106,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,hydroxylation,+O,C13H18O3
T107,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,methane addition,+CH4,C14H22O2
T108,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,dihydroxylation,+2O,C13H18O4
T109,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,sulfation,+S,C13H18O2S
T110,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,trioxidation,+3O,C13H18O5
T111,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,dehydrogenation,-H2,C13H16O2
T112,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,oxidation to carbonyl,+O -2H,C13H16O3
T113,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,methylation,+CH2,C14H20O2
T114,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,decarboxylation,-CO2,C12H18
T115,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,loss of ethylene oxide,-C2H4O,C11H14O
T116,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,major dealkylation,-C4H8,C9H10O2
T117,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,loss of acrolein,-C3H4O,C10H14O
T118,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,decarbonylation,-CO,C12H18O
T119,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,dealkylation,-C2H4,C11H14O2
T120,ibuprofen,CC(C)Cc1ccc(C(C)C(=O)O)cc1,C13H18O2,net glucuronidation,+C6H8O6,C19H26O8"""

# -----------------------------------------------------------------------------
# Deterministic specs for isomers and different formulas
# -----------------------------------------------------------------------------
ISOMERS_SPEC = [
    ("carbamazepine", "C15H14N2O2", [2.30, 3.50]),
    ("paracetamol", "C7H7NO2", [1.50, 2.15]),
    ("caffeine", "C8H8N4O2", [2.20, 2.95, 3.60]),
    ("atrazine", "C6H10ClN5", [3.10, 3.85]),
    ("diclofenac", "C14H12ClNO2", [3.75, 4.30, 4.95]),
    ("ibuprofen", "C13H16O2", [4.10, 4.70, 5.25]),
    ("sulfamethoxazole", "C10H9N3O3S", [2.80, 3.45]),
    ("flufenacet", "C14H11F4N3OS", [4.80, 5.25, 5.90]),
]

DIFF_SPEC = [
    # Triplets (same nominal m/z, 3 different formulas)
    ("carbamazepine", 253, [("C14H8N2O3", 3.80), ("C15H12N2O2", 4.35), ("C16H16N2O", 5.60)]),
    ("caffeine", 181, [("C6H4N4O3", 1.90), ("C7H8N4O2", 2.65), ("C8H12N4O", 3.45)]),
    # Pairs (with examples of inverse RT order)
    ("carbamazepine", 209, [("C13H8N2O", 4.20), ("C14H12N2", 3.10)]),      # INVERSE RT
    ("carbamazepine", 251, [("C15H10N2O2", 3.80), ("C16H14N2O", 4.70)]),
    ("carbamazepine", 269, [("C15H12N2OS", 4.90), ("C15H12N2O3", 3.30)]),    # INVERSE RT
    ("paracetamol", 122, [("C7H7NO", 1.80), ("C8H9O", 2.45)]),
    ("paracetamol", 124, [("C6H5NO2", 3.60), ("C7H9NO", 2.10)]),           # INVERSE RT
    ("paracetamol", 168, [("C8H9NO3", 1.75), ("C9H13NO2", 2.55)]),
    ("paracetamol", 184, [("C8H9NO2S", 2.85), ("C8H9NO4", 1.95)]),         # INVERSE RT
    ("caffeine", 165, [("C7H8N4O", 2.05), ("C8H10N3O", 2.70)]),
    ("caffeine", 167, [("C6H6N4O2", 1.90), ("C7H10N4O", 2.60)]),
    ("atrazine", 230, [("C8H12ClN5O", 3.60), ("C9H16ClN5", 4.40)]),
    ("atrazine", 232, [("C8H14ClN5O", 3.40), ("C9H18ClN5", 4.20)]),
    ("diclofenac", 252, [("C12H7Cl2NO", 4.10), ("C13H11Cl2N", 4.90)]),
    ("diclofenac", 278, [("C14H9Cl2NO", 4.85), ("C14H12ClNO3", 3.70)]),     # INVERSE RT
    ("sulfamethoxazole", 190, [("C5H7N3O3S", 1.65), ("C10H11N3O", 2.50)]),
    ("ibuprofen", 151, [("C9H10O2", 3.20), ("C10H14O", 3.90)]),
    ("flufenacet", 322, [("C11H7F4N3O2S", 4.40), ("C12H11F4N3OS", 5.10)]),
]


def get_versions() -> tuple[str, str]:
    """Returns the versions of Python and pyteomics."""
    py_ver = sys.version.split()[0]
    try:
        pyt_ver = importlib.metadata.version("pyteomics")
    except Exception:
        pyt_ver = getattr(pyteomics, "__version__", "5.0.1")
    return py_ver, pyt_ver


def generate_transformations_bytes() -> bytes:
    """Generates the exact bytes of trasformazioni.csv, deterministically."""
    lines = [line.strip("\r\n") for line in TRANSFORMATIONS_RAW.strip().splitlines()]
    content = "\r\n".join(lines) + "\r\n"
    return content.encode("utf-8")


def build_name_cases_records(t_rows: list[dict[str, str]]) -> list[dict]:
    """Builds the 110 test cases from the naming specs."""
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
    singles_spec.append(("ibuprofen", "C19H26O8", "[M-H]-", 3.65))
    singles_spec.append(("ibuprofen", "C13H18O5", "[M-H]-", 2.90))

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


def generate_name_cases_bytes(records: list[dict]) -> bytes:
    """Generates the exact bytes of casi_nomi.csv (CRLF, UTF-8)."""
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
    """Checks the byte-for-byte match when the file exists, otherwise writes it."""
    if path.exists():
        existing_bytes = path.read_bytes()
        if existing_bytes != new_bytes:
            raise ValueError(f"Discrepanza byte-per-byte per {label}: {len(existing_bytes)} vs {len(new_bytes)}")
        print(f"  [OK] {label}: byte-for-byte identico al file esistente ({len(new_bytes)} byte).")
    else:
        path.write_bytes(new_bytes)
        print(f"  [OK] {label}: creato nuovo file deterministico ({len(new_bytes)} byte).")


def cleanup_obsolete_scripts(target_dir: Path) -> None:
    """Removes old scratch scripts left by earlier iterations."""
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
    transf_bytes = generate_transformations_bytes()
    transf_path = target_dir / "trasformazioni.csv"
    verify_and_write(transf_path, transf_bytes, "trasformazioni.csv")

    # 2. Load the transformation rows and generate casi_nomi.csv
    reader = csv.DictReader(io.StringIO(transf_bytes.decode("utf-8")))
    t_rows = list(reader)
    cases_records = build_name_cases_records(t_rows)
    cases_bytes = generate_name_cases_bytes(cases_records)
    cases_path = target_dir / "casi_nomi.csv"
    verify_and_write(cases_path, cases_bytes, "casi_nomi.csv")

    # 3. Clean up obsolete scripts
    cleanup_obsolete_scripts(target_dir)

    print("-----------------------------------------------------------------")
    print(f"Rigenerazione deterministica completata con successo:")
    print(f"  - trasformazioni.csv: {len(t_rows)} righe, 10864 byte")
    print(f"  - casi_nomi.csv:      {len(cases_records)} righe, 9266 byte")
    print("=================================================================")


if __name__ == "__main__":
    main()
