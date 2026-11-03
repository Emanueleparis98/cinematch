/* Minimal ZIP reader for the self-updater. It supports the deflate/raw entries
   produced by scripts/build-extension.py and rejects unsafe paths and ZIP64. */
const EOCD = 0x06054b50;
const CENTRAL = 0x02014b50;
const LOCAL = 0x04034b50;
const MAX_FILES = 2000;
const MAX_UNCOMPRESSED = 256 * 1024 * 1024;

const crcTable = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) crc = crcTable[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function safePath(path) {
  if (!path || path.length > 300 || path.includes("\\") || path.includes("\0")) return false;
  if (path.startsWith("/")) return false;
  const parts = path.split("/");
  return parts.every(part => part && part !== "." && part !== "..");
}

function decodeName(bytes, start, length) {
  return new TextDecoder().decode(bytes.subarray(start, start + length));
}

async function inflateRaw(bytes) {
  if (typeof DecompressionStream !== "function") throw new Error("Questo browser non supporta l’estrazione ZIP.");
  const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

export async function extractZip(input, options = {}) {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  if (bytes.length < 22) throw new Error("ZIP troppo piccolo o danneggiato.");
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let eocd = -1;
  const min = Math.max(0, bytes.length - 22 - 0xffff);
  for (let i = bytes.length - 22; i >= min; i -= 1) {
    if (view.getUint32(i, true) === EOCD) { eocd = i; break; }
  }
  if (eocd < 0) throw new Error("Indice ZIP non trovato.");
  const count = view.getUint16(eocd + 10, true);
  const centralOffset = view.getUint32(eocd + 16, true);
  if (count === 0xffff || centralOffset === 0xffffffff) throw new Error("Archivi ZIP64 non supportati.");
  if (count > (options.maxFiles ?? MAX_FILES)) throw new Error("Il pacchetto contiene troppi file.");

  const files = [];
  let offset = centralOffset;
  let total = 0;
  for (let index = 0; index < count; index += 1) {
    if (offset + 46 > bytes.length || view.getUint32(offset, true) !== CENTRAL) throw new Error("Indice ZIP non valido.");
    const method = view.getUint16(offset + 10, true);
    const crc = view.getUint32(offset + 16, true);
    const compressedSize = view.getUint32(offset + 20, true);
    const uncompressedSize = view.getUint32(offset + 24, true);
    const nameLength = view.getUint16(offset + 28, true);
    const extraLength = view.getUint16(offset + 30, true);
    const commentLength = view.getUint16(offset + 32, true);
    const localOffset = view.getUint32(offset + 42, true);
    const name = decodeName(bytes, offset + 46, nameLength);
    offset += 46 + nameLength + extraLength + commentLength;
    if (name.endsWith("/")) continue;
    if (!safePath(name)) throw new Error(`Percorso ZIP non sicuro: ${name}`);
    if (method !== 0 && method !== 8) throw new Error(`Compressione ZIP non supportata per ${name}.`);
    if (localOffset + 30 > bytes.length || view.getUint32(localOffset, true) !== LOCAL) throw new Error(`Header ZIP non valido per ${name}.`);
    const localNameLength = view.getUint16(localOffset + 26, true);
    const localExtraLength = view.getUint16(localOffset + 28, true);
    const dataStart = localOffset + 30 + localNameLength + localExtraLength;
    if (dataStart + compressedSize > bytes.length) throw new Error(`Dati ZIP troncati per ${name}.`);
    const compressed = bytes.subarray(dataStart, dataStart + compressedSize);
    const data = method === 8 ? await inflateRaw(compressed) : compressed.slice();
    if (data.length !== uncompressedSize) throw new Error(`Dimensione non valida per ${name}.`);
    if (crc32(data) !== crc) throw new Error(`CRC non valido per ${name}.`);
    total += data.length;
    if (total > (options.maxBytes ?? MAX_UNCOMPRESSED)) throw new Error("Il pacchetto supera la dimensione massima consentita.");
    files.push({ path: name, data });
  }
  return files;
}
