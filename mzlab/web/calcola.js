// @ts-check
// @ts-strict
"use strict";
// Arithmetic of the header calculator (block 4.2): pure functions, also run by node in tests/test_calcola.py. No eval (the page has a strict CSP):
// a small recursive-descent parser. The field takes an EXPRESSION (only digits, decimal point or comma, + - x / ( ) and "Ans") or a chemical formula.

// true when the text is an arithmetic expression (so: not a formula); the text may be empty
function calcIsExpr(t) {
  const s = String(t || "").replace(/ans/gi, "").replace(/\s+/g, "");
  return /^[0-9.,+\-−–—*xX×/÷()]*$/.test(s) && /[0-9]|ans/i.test(String(t || ""));
}
// evaluates; ans = last result (for "Ans"). Returns { v } or { err } (err = short Italian message, "" while the expression is only unfinished)
function calcEval(t, ans = 0) {
  let s = String(t || "").replace(/ans/gi, `(${ans})`).replace(/\s+/g, "").replace(/,/g, ".").replace(/[−–—]/g, "-").replace(/[xX×]/g, "*").replace(/÷/g, "/");
  if (!s) return { err: "" };
  let i = 0;
  const peek = () => s[i], num = () => {
    const m = /^\d*\.?\d*/.exec(s.slice(i)); if (!m || !m[0] || m[0] === ".") return null;
    i += m[0].length; return parseFloat(m[0]);
  };
  const fail = m => { throw { err: m }; };
  function factor() {
    const c = peek();
    if (c === "-") { i++; return -factor(); }
    if (c === "+") { i++; return factor(); }
    if (c === "(") { i++; const v = expr(); if (peek() !== ")") fail(""); i++; return v; }
    const n = num(); if (n == null) fail(i >= s.length ? "" : I18N.t("calc.err.sign")); return n;
  }
  function term() {
    let v = factor();
    while (peek() === "*" || peek() === "/") { const o = s[i++], r = factor(); if (o === "/" && r === 0) fail("divisione per zero"); v = o === "*" ? v * r : v / r; }
    return v;
  }
  function expr() {
    let v = term();
    while (peek() === "+" || peek() === "-") { const o = s[i++], r = term(); v = o === "+" ? v + r : v - r; }
    return v;
  }
  try { const v = expr(); if (i < s.length) fail(peek() === ")" ? I18N.t("calc.err.closeParen") : ""); return isFinite(v) ? { v } : { err: I18N.t("calc.err.tooBig") }; }
  catch (e) { return e && "err" in e ? e : { err: "" }; }
}
// 4 decimals at most, trailing zeros removed, never scientific notation for normal numbers
function calcFmt(v) {
  if (!isFinite(v)) return "";
  const a = Math.abs(v);
  if (a >= 1e15) return String(v);
  let t = v.toFixed(4).replace(/(\.\d*?)0+$/, "$1").replace(/\.$/, "");
  if (t === "-0") t = "0";
  return t;
}
// the expression as shown in the tape: Italian signs, spaces around the operators
function calcPretty(t) { return String(t || "").replace(/\s+/g, "").replace(/\*|x|X|×/g, " × ").replace(/\//g, " ÷ ").replace(/(?<=[0-9.,)])\+/g, " + ").replace(/(?<=[0-9.,)])[-−]/g, " − ").trim(); }
if (typeof module !== "undefined") module.exports = { calcIsExpr, calcEval, calcFmt, calcPretty };
