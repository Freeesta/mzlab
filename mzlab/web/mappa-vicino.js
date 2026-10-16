// @ts-check
// Previous / next file of the 2D map (the ◀ ▶ arrows next to the file menu): pure function, tested in Node (tests/mappa_vicino_node.js). Classic script, loaded before pannelli.
const MAPVICINO = (() => {
  /** The file `dir` steps (-1 before, +1 after) from `cur` in the menu order, jumping over the reference of the difference (a file minus itself is empty); null when there is none.
   * @param {number[]} keys file keys in the order of the menu @param {number} cur the file shown @param {number} dir -1 | 1 @param {number | null | undefined} ref the file subtracted, if any
   * @returns {number | null} */
  const vicino = (keys, cur, dir, ref) => {
    const i = keys.indexOf(cur); if (i < 0) return null;
    for (let j = i + dir; j >= 0 && j < keys.length; j += dir) if (keys[j] !== ref) return keys[j];
    return null;
  };
  return { vicino };
})();
