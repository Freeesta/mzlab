# Study script (docs/studi/CC_MOTORE_NEL_BROWSER.md, section 3), not part of the program. Needs a venv with tblite 0.7.0, rdkit, numpy.
import sys, json
from rdkit import Chem
def classes(smi, ranks, f):
    m=Chem.AddHs(Chem.MolFromSmiles(smi)); g={}
    for a in m.GetAtoms():
        if a.GetSymbol()=="C" and a.GetTotalNumHs(includeNeighbors=True)>0 and (a.GetIsAromatic() or a.GetHybridization()==Chem.HybridizationType.SP2):
            g.setdefault(ranks[a.GetIdx()],[]).append(a.GetIdx())
    return sorted(((sum(f[i] for i in v)/len(v), tuple(v)) for v in g.values()), reverse=True)
def run(data, key):
    res={}
    for n,r in data.items():
        res[n]=classes(r["smiles"], r["ranks"], key(r))
    return res
if __name__=="__main__":
    d=json.load(open(sys.argv[1]))
    for lab,key in [("alpb f-",lambda r:r["alpb"]["fm"]),("alpb f0",lambda r:[(a+b)/2 for a,b in zip(r["alpb"]["fm"],r["alpb"]["fp"])]),("gas f-",lambda r:r["gas"]["fm"])]:
        print("==",lab)
        for n,c in run(d,key).items(): print(f"{n:14s}", "  ".join(f"{list(v)}:{x:.3f}" for x,v in c))
