// Reads a structure file dropped on the Drawing canvas for Ketcher's own reader (Indigo): text files as text, binary CDX (starts with "VjCD") as base64,
// the form Ketcher's Open dialog gives it. No format is parsed here.
const CDX_MAGIC = [0x56, 0x6a, 0x43, 0x44];

/** @param {File} file @returns {Promise<string>} */
export async function structureText(file) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  if (CDX_MAGIC.every((b, i) => bytes[i] === b)) {
    let bin = "";
    for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    return btoa(bin);
  }
  return new TextDecoder().decode(bytes);
}
