// The display form of molecule names of the Fragmentation graph (mzlab/web/infusione-grafo-nome.js). Prints «ok»; exit code 1 on the first failure.
const assert = require("assert"), path = require("path"), { pathToFileURL } = require("url");
import(pathToFileURL(path.resolve(__dirname, "../mzlab/web/infusione-grafo-nome.js")).href).then(({ nomeMaiuscolo: f }) => {
  const cases = [["timolol", "Timolol"], ["Timolol", "Timolol"], ["", ""], ["2-aminophenol", "2-aminophenol"], ["N-acetylcysteine", "N-acetylcysteine"],
    ["(R)-timolol", "(R)-timolol"], ["α-pinene", "α-pinene"], ["β-estradiol", "β-estradiol"], ["p-cresol", "p-cresol"], ["tert-butanol", "tert-butanol"],
    ["cis-platin", "cis-platin"], ["rac-ibuprofen", "rac-ibuprofen"], ["carbamazepine epoxide", "Carbamazepine epoxide"], ["éthanol", "éthanol"]];
  for (const [a, b] of cases) assert.strictEqual(f(a), b, a);
  console.log("ok");
}).catch(e => { console.error(e); process.exit(1); });
