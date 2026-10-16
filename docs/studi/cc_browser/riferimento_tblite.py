# Study script (docs/studi/CC_MOTORE_NEL_BROWSER.md, section 3), not part of the program. Needs a venv with tblite 0.7.0, rdkit, numpy.
import sys, json, time, numpy as np
from rdkit import Chem
from rdkit.Chem import AllChem
from tblite.interface import Calculator
MOLS = {"phenol":"Oc1ccccc1","anisole":"COc1ccccc1","toluene":"Cc1ccccc1","benzoic_acid":"OC(=O)c1ccccc1",
 "nitrobenzene":"[O-][N+](=O)c1ccccc1","chlorobenzene":"Clc1ccccc1","aniline":"Nc1ccccc1","naphthalene":"c1ccc2ccccc2c1",
 "carbamazepine":"NC(=O)N1c2ccccc2C=Cc2ccccc21","paracetamol":"CC(=O)Nc1ccc(O)cc1"}
B=1/0.52917721092
def charges(Z,X,q,uhf,solv):
    c=Calculator("GFN2-xTB",Z,X*B,charge=q,uhf=uhf); c.set("verbosity",0)
    if solv: c.add("alpb-solvation","water")
    return c.singlepoint().get("charges")
out={}
for name,smi in MOLS.items():
    m=Chem.AddHs(Chem.MolFromSmiles(smi)); AllChem.EmbedMolecule(m,randomSeed=42); AllChem.MMFFOptimizeMolecule(m)
    Z=np.array([a.GetAtomicNum() for a in m.GetAtoms()]); X=m.GetConformer().GetPositions()
    nel=int(Z.sum()); uhf=1 if nel%2==0 else 0
    r={"smiles":smi,"Z":Z.tolist(),"xyz":X.tolist(),"ranks":list(Chem.CanonicalRankAtoms(m,breakTies=False))}
    for solv in (False,True):
        t=time.time(); q0=charges(Z,X,0,0,solv); qc=charges(Z,X,1,uhf,solv); qa=charges(Z,X,-1,uhf,solv)
        r["alpb" if solv else "gas"]={"q0":q0.tolist(),"fm":(qc-q0).tolist(),"fp":(q0-qa).tolist(),"s":time.time()-t}
    out[name]=r
json.dump(out,open(sys.argv[1],"w"))
print({k:(round(v["gas"]["s"],2),round(v["alpb"]["s"],2)) for k,v in out.items()})
