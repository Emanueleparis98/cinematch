import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {deflateRawSync} from 'node:zlib';
import {extractZip} from '../extensions/cinematch/zip-reader.js';

const manifest = JSON.parse(await readFile('extensions/cinematch/manifest.json', 'utf8'));
const archive = await readFile(`dist/CineMatch-Chrome-${manifest.version}.zip`);
const files = await extractZip(archive);
const byName = new Map(files.map(file => [file.path, file.data]));
assert.ok(byName.has('manifest.json'), 'manifest');
assert.ok(byName.has('launcher.js'), 'launcher');
assert.ok(byName.has('panel.html'), 'panel');
assert.ok(byName.has('updater.js'), 'updater');
assert.ok(byName.has('zip-reader.js'), 'zip reader');
assert.ok(byName.has('data/movements.json'), 'movements');
assert.equal(JSON.parse(new TextDecoder().decode(byName.get('manifest.json'))).version, manifest.version);
assert.ok(files.length > 100, 'full package');

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
function makeZip(entries) {
  const locals = [];
  const centrals = [];
  let offset = 0;
  for (const [name, text] of Object.entries(entries)) {
    const nameBytes = Buffer.from(name);
    const data = Buffer.from(text);
    const compressed = deflateRawSync(data);
    const crc = crc32(data);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(0x0800, 6);
    local.writeUInt16LE(8, 8);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(compressed.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(nameBytes.length, 26);
    const localRecord = Buffer.concat([local, nameBytes, compressed]);
    locals.push(localRecord);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt16LE(0x0800, 8);
    central.writeUInt16LE(8, 10);
    central.writeUInt32LE(crc, 16);
    central.writeUInt32LE(compressed.length, 20);
    central.writeUInt32LE(data.length, 24);
    central.writeUInt16LE(nameBytes.length, 28);
    central.writeUInt32LE(offset, 42);
    centrals.push(Buffer.concat([central, nameBytes]));
    offset += localRecord.length;
  }
  const central = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(centrals.length, 8);
  end.writeUInt16LE(centrals.length, 10);
  end.writeUInt32LE(central.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, central, end]);
}

const safe = makeZip({'manifest.json': '{"name":"CineMatch","version":"1.0.0"}', 'folder/file.txt': 'ok'});
const safeFiles = await extractZip(safe);
assert.deepEqual(safeFiles.map(file => file.path).sort(), ['folder/file.txt', 'manifest.json']);
const unsafe = makeZip({'../escape.txt': 'no'});
await assert.rejects(() => extractZip(unsafe), /non sicuro/);
console.log('ZIP reader checks passed.');
