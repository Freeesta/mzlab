# Study script (docs/studi/CC_MOTORE_NEL_BROWSER.md, section 3), not part of the program. Needs a venv with tblite 0.7.0, rdkit, numpy.
import sys, json, numpy as np
sys.path.insert(0, sys.argv[3]); from classi import classes
ref=json.load(open(sys.argv[1])); sf=json.load(open(sys.argv[2]))
for n,r in ref.items():
    s=sf[n]; q0r=np.array(r["gas"]["q0"]); q0s=np.array(s["q0"])
    print(f"{n:14s} maxΔq0={abs(q0r-q0s).max():.4f} maxΔf-={abs(np.array(r['gas']['fm'])-np.array(s['fm'])).max():.4f} conv={s['conv']} gfn2x3={s['ms']:.0f}ms eht={s['ms_eht']:.1f}ms emb={s['ms_emb']:.0f}ms ok={s['emb_ok']} n={s['emb_n']}/{len(r['Z'])}")
    for lab,f in [("sf gfn2 gas f-",s["fm"]),("sf EHT f-",s["eht_fm"])]:
        print("    ",lab.ljust(15),"  ".join(f"{list(v)}:{x:.3f}" for x,v in classes(r["smiles"],r["ranks"],f)))
