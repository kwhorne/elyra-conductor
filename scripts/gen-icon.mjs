// Generates a simple solid-color PNG icon without external deps.
import { deflateSync } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";

const size = 1024;
const [r, g, b] = [0x7a, 0xa2, 0xf7]; // accent blue
const bg = [0x16, 0x16, 0x1e];

// Build raw RGBA pixels: blue rounded-ish square on dark bg with a chevron.
const px = Buffer.alloc(size * size * 4);
function set(x, y, c) {
  const i = (y * size + x) * 4;
  px[i] = c[0];
  px[i + 1] = c[1];
  px[i + 2] = c[2];
  px[i + 3] = 255;
}
for (let y = 0; y < size; y++) {
  for (let x = 0; x < size; x++) set(x, y, bg);
}
// chevron ">" centered
const cx = size * 0.4;
const cy = size / 2;
const thick = size * 0.09;
for (let y = 0; y < size; y++) {
  for (let x = 0; x < size; x++) {
    const dy = Math.abs(y - cy);
    const armX = cx + dy; // diagonal
    if (Math.abs(x - armX) < thick && dy < size * 0.22) set(x, y, [r, g, b]);
  }
}

// PNG encoding
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const t = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])), 0);
  return Buffer.concat([len, t, data, crc]);
}
const crcTable = (() => {
  const t = [];
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = crcTable[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(size, 0);
ihdr.writeUInt32BE(size, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA
// add filter byte 0 per row
const raw = Buffer.alloc((size * 4 + 1) * size);
for (let y = 0; y < size; y++) {
  raw[y * (size * 4 + 1)] = 0;
  px.copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4);
}
const idat = deflateSync(raw);
const png = Buffer.concat([
  sig,
  chunk("IHDR", ihdr),
  chunk("IDAT", idat),
  chunk("IEND", Buffer.alloc(0)),
]);

mkdirSync("src-tauri/icons", { recursive: true });
writeFileSync("src-tauri/icons/icon.png", png);
console.log("wrote src-tauri/icons/icon.png", png.length, "bytes");
