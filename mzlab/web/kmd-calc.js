// @ts-strict
"use strict";
// Repeating units for Kendrick mass defect analysis (exact monoisotopic masses, C = 12 by definition).
(function (root) {
  const UNITS = [
    { id: "CF2", mass: 49.99680632546, nominal: 50 },
    { id: "CF2O", mass: 65.99172094503, nominal: 66 },
    { id: "C2F4", mass: 99.99361265092, nominal: 100 },
    { id: "CH2", mass: 14.01565006446, nominal: 14 }
  ];
  const unitById = id => UNITS.find(u => u.id === id) || UNITS[0];

  const api = { UNITS, unitById };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.KMDCALC = api;
})(typeof window !== "undefined" ? window : globalThis);
