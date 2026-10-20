// The Review view of Flow (flow/lazy/flow-review.js without the DOM): the queue, the level written by Accept, the notes the engine will ask and the decision it reads. Exit code 1 on the first failure.
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert");
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const ctx = vm.createContext({ window: {}, document: {}, localStorage: {} });
ctx.FLOW = {};
ctx.globalThis = ctx;
vm.runInContext(fs.readFileSync(path.resolve(__dirname, "../flow/lazy/flow-review.js"), "utf8"), ctx);
const F = ctx.FLOW;
// only the rows that wait for a decision (2b, 3a, 3b) enter the queue, in the order of the table
eq(F.reviewQueue({ columns: [], rows: [["POS", 1, 100, 1, 1, ""], ["POS", 2, 101, 2, 2, "b"], ["NEG", 3, 102, 3, 3, "a"], ["POS", 4, 103, 4, 2, "a"], ["POS", 5, 104, 5, 3, "b"], ["POS", 6, 105, 6, 0, ""]] }), [{ pol: "POS", id: 2 }, { pol: "NEG", id: 3 }, { pol: "POS", id: 5 }]);
// "1" is no decision: the cascade's level stays
eq(["1", "2a", "2b", "3"].map(F.reviewLevel), ["", "2a", "2b", "3"]);
// a note is asked when the level or the identity changes
eq(F.reviewChanges({ level: "2", group: "" }, "2b", ""), ["level 2 -> 2b"]);
eq(F.reviewChanges({ level: "3", group: "" }, "3b", ""), []);
eq(F.reviewChanges({ level: "3", group: "" }, "", "ambiguous"), ["identity rejected"]);
eq(F.reviewChanges({ level: "3", group: "ambiguous" }, "", "ambiguous"), []);
// the decision the engine reads: the reviewer and the note, the explained fragments as evidence, nothing ticked
const d = F.reviewDecision({ pol: "POS", id: 15, proposal: { evidence: { n_explained: 4, frac_explained: 0.8 } } }, { level: "2b", group: "", note: "seen", by: "ab" });
eq([d.pol, d.id, d.level, d.justification, d.decided_by, d.evidence.type, d.evidence.matched, d.checklist], ["POS", 15, "2b", "seen", "ab", "fragments", 4, []]);
eq(F.reviewDecision({ pol: "POS", id: 1, proposal: { evidence: {} } }, { level: "", group: "ambiguous", note: "n", by: "ab" }).evidence.type, "");
console.log("ok");
