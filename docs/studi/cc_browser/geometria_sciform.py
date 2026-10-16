# Study script (docs/studi/CC_MOTORE_NEL_BROWSER.md, section 3), not part of the program. Needs a venv with tblite 0.7.0, rdkit, numpy.
import sys, json, numpy as np
sys.path.insert(0, sys.argv[3]); from classi import classes
from tblite.interface import Calculator
ref=json.load(open(sys.argv[1])); sf=json.load(open(sys.argv[2])); B=1/0.52917721092
def ch(Z,X,q,u):
    c=Calculator("GFN2-xTB",Z,X*B,charge=q,uhf=u); c.set("verbosity",0); c.add("alpb-solvation","water"); return c.singlepoint().get("charges")
for n,r in ref.items():
    s=sf[n]; Z=np.array(s["emb_el"]); 
    if list(Z)!=r["Z"]: print(n,"atom order differs"); continue
    X=np.array(s["emb_xyz"]).reshape(-1,3); u=1 if Z.sum()%2==0 else 0
    fm=ch(Z,X,1,u)-ch(Z,X,0,0)
    a=classes(r["smiles"],r["ranks"],r["alpb"]["fm"]); b=classes(r["smiles"],r["ranks"],fm.tolist())
    print(f"{n:14s} same_order={[v for _,v in a]==[v for _,v in b]}  sf-geom:", "  ".join(f"{list(v)}:{x:.3f}" for x,v in b))
