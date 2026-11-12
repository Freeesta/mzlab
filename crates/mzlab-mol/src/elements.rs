//! The element symbols a record may carry (H to Og); the check is on the symbol only, no valence rules.

const SYMBOLS: &str = "H He Li Be B C N O F Ne Na Mg Al Si P S Cl Ar K Ca Sc Ti V Cr Mn Fe Co Ni Cu Zn Ga Ge As Se Br Kr Rb Sr Y Zr Nb Mo Tc Ru Rh Pd Ag Cd In Sn Sb Te I Xe Cs Ba La Ce Pr Nd Pm Sm Eu Gd Tb Dy Ho Er Tm Yb Lu Hf Ta W Re Os Ir Pt Au Hg Tl Pb Bi Po At Rn Fr Ra Ac Th Pa U Np Pu Am Cm Bk Cf Es Fm Md No Lr Rf Db Sg Bh Hs Mt Ds Rg Cn Nh Fl Mc Lv Ts Og";

/// True when `s` is the symbol of a chemical element (exact case).
pub fn is_element(s: &str) -> bool {
    atomic_number(s).is_some()
}

/// The atomic number of an element symbol (exact case), `None` for anything else.
pub fn atomic_number(s: &str) -> Option<u16> {
    SYMBOLS
        .split(' ')
        .position(|e| e == s)
        .map(|p| p as u16 + 1)
}
