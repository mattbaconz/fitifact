/**
 * Container-level stream census and SHA-256 for the browser video path.
 *
 * Mediabunny lists only the tracks it can decode or copy, so the engine would never see a
 * timecode, metadata, subtitle, or attachment stream it silently drops. This walks the container
 * itself, as ffprobe does, so the planner refuses those files exactly like the desktop app.
 */

export type CensusTrack = "video" | "audio" | "subtitle" | "data" | "attachment";

export interface Census {
  container: "isobmff" | "matroska";
  majorBrand: string | null;
  tracks: CensusTrack[];
}

export class CensusFailure extends Error {}

type Read = (start: number, end: number) => Promise<Uint8Array>;

// ponytail: the whole moov is read at once; a streaming box walk if real files outgrow this.
const MAX_MOOV_BYTES = 64 * 1024 * 1024;
const MAX_TRACKS_BYTES = 16 * 1024 * 1024;

const HANDLERS: Record<string, CensusTrack> = {
  vide: "video",
  soun: "audio",
  text: "subtitle",
  sbtl: "subtitle",
  subt: "subtitle",
  clcp: "subtitle",
  subp: "subtitle",
};

export async function trackCensus(blob: Blob): Promise<Census> {
  const read: Read = async (start, end) =>
    new Uint8Array(await blob.slice(start, Math.min(end, blob.size)).arrayBuffer());
  const head = await read(0, 8);
  if (head.length === 8 && ascii(head, 4, 8) === "ftyp") return isobmffCensus(read, blob.size);
  if (head[0] === 0x1a && head[1] === 0x45 && head[2] === 0xdf && head[3] === 0xa3) {
    return matroskaCensus(read, blob.size);
  }
  throw new CensusFailure("Not an MP4, MOV, MKV, or WebM file.");
}

async function isobmffCensus(read: Read, size: number): Promise<Census> {
  let majorBrand: string | null = null;
  let offset = 0;
  while (offset + 8 <= size) {
    const header = await read(offset, offset + 16);
    const box = boxHeader(header, 0, size - offset);
    if (box.type === "ftyp" && header.length >= 12) majorBrand = ascii(header, 8, 12);
    if (box.type === "moov") {
      if (box.size > MAX_MOOV_BYTES) throw new CensusFailure("The movie index is too large to read here.");
      const moov = await read(offset + box.header, offset + box.size);
      const tracks = boxes(moov)
        .filter((child) => child.type === "trak")
        .map((trak) => HANDLERS[handlerType(trak.body)] ?? "data");
      return { container: "isobmff", majorBrand, tracks };
    }
    offset += box.size;
  }
  throw new CensusFailure("The file has no movie index.");
}

function handlerType(trak: Uint8Array): string {
  const mdia = boxes(trak).find((child) => child.type === "mdia");
  const hdlr = mdia && boxes(mdia.body).find((child) => child.type === "hdlr");
  return hdlr && hdlr.body.length >= 12 ? ascii(hdlr.body, 8, 12) : "";
}

function boxHeader(bytes: Uint8Array, offset: number, remaining: number) {
  if (offset + 8 > bytes.length) throw new CensusFailure("A box header is cut off.");
  const view = new DataView(bytes.buffer, bytes.byteOffset + offset);
  const type = ascii(bytes, offset + 4, offset + 8);
  let size = view.getUint32(0);
  let header = 8;
  if (size === 1) {
    if (offset + 16 > bytes.length) throw new CensusFailure("A box header is cut off.");
    size = Number(view.getBigUint64(8));
    header = 16;
  } else if (size === 0) {
    size = remaining;
  }
  if (size < header || size > remaining) throw new CensusFailure("A box size is out of range.");
  return { type, size, header };
}

function boxes(bytes: Uint8Array): Array<{ type: string; body: Uint8Array }> {
  const found = [];
  for (let offset = 0; offset + 8 <= bytes.length; ) {
    const box = boxHeader(bytes, offset, bytes.length - offset);
    found.push({ type: box.type, body: bytes.subarray(offset + box.header, offset + box.size) });
    offset += box.size;
  }
  return found;
}

const SEGMENT = 0x18538067;
const CLUSTER = 0x1f43b675;
const TRACKS = 0x1654ae6b;
const TRACK_ENTRY = 0xae;
const TRACK_TYPE = 0x83;
const ATTACHMENTS = 0x1941a469;
const ATTACHED_FILE = 0x61a7;

interface Element {
  id: number;
  start: number;
  end: number;
  unknownSize: boolean;
}

async function matroskaCensus(read: Read, size: number): Promise<Census> {
  const ebml = await elementAt(read, 0, size);
  const segment = await elementAt(read, ebml.end, size);
  if (segment.id !== SEGMENT) throw new CensusFailure("The Matroska segment is missing.");
  let tracks: CensusTrack[] | null = null;
  let attachments = 0;
  for (let offset = segment.start; offset < segment.end; ) {
    const child = await elementAt(read, offset, segment.end);
    if (child.unknownSize) {
      // A live recording ends in open-ended clusters; everything else was written before them.
      if (child.id === CLUSTER && tracks) break;
      throw new CensusFailure("The file is still open-ended or damaged.");
    }
    if (child.id === TRACKS) {
      if (child.end - child.start > MAX_TRACKS_BYTES) throw new CensusFailure("The track list is too large.");
      tracks = elements(await read(child.start, child.end))
        .filter((entry) => entry.id === TRACK_ENTRY)
        .map((entry) => matroskaTrackType(entry.body));
    } else if (child.id === ATTACHMENTS) {
      attachments += elements(await read(child.start, child.end)).filter((item) => item.id === ATTACHED_FILE).length;
    }
    offset = child.end;
  }
  if (!tracks) throw new CensusFailure("The file has no track list.");
  return {
    container: "matroska",
    majorBrand: null,
    tracks: [...tracks, ...Array<CensusTrack>(attachments).fill("attachment")],
  };
}

function matroskaTrackType(entry: Uint8Array): CensusTrack {
  const type = elements(entry).find((child) => child.id === TRACK_TYPE)?.body;
  const value = type?.reduce((total, byte) => total * 256 + byte, 0);
  return value === 1 ? "video" : value === 2 ? "audio" : value === 0x11 ? "subtitle" : "data";
}

async function elementAt(read: Read, offset: number, limit: number): Promise<Element> {
  const bytes = await read(offset, Math.min(offset + 12, limit));
  const element = elementHeader(bytes, 0);
  const start = offset + element.headerLength;
  const end = element.unknownSize ? limit : start + element.size;
  if (end > limit) throw new CensusFailure("An element runs past its parent.");
  return { id: element.id, start, end, unknownSize: element.unknownSize };
}

function elements(bytes: Uint8Array): Array<{ id: number; body: Uint8Array }> {
  const found = [];
  for (let offset = 0; offset < bytes.length; ) {
    const element = elementHeader(bytes, offset);
    const start = offset + element.headerLength;
    const end = start + element.size;
    if (element.unknownSize || end > bytes.length) throw new CensusFailure("An element runs past its parent.");
    found.push({ id: element.id, body: bytes.subarray(start, end) });
    offset = end;
  }
  return found;
}

function elementHeader(bytes: Uint8Array, offset: number) {
  const id = vint(bytes, offset, true);
  const size = vint(bytes, offset + id.length, false);
  return { id: id.value, size: size.value, headerLength: id.length + size.length, unknownSize: size.allOnes };
}

/** EBML variable-length integer; IDs keep their length marker, sizes drop it. */
function vint(bytes: Uint8Array, offset: number, keepMarker: boolean) {
  const first = bytes[offset];
  if (first === undefined || first === 0) throw new CensusFailure("An element header is damaged.");
  const length = Math.clz32(first) - 23;
  if (offset + length > bytes.length) throw new CensusFailure("An element header is cut off.");
  const mask = 0xff >> length;
  let value = keepMarker ? first : first & mask;
  let allOnes = (first & mask) === mask;
  for (let index = 1; index < length; index++) {
    value = value * 256 + bytes[offset + index];
    allOnes &&= bytes[offset + index] === 0xff;
  }
  return { value, length, allOnes };
}

function ascii(bytes: Uint8Array, start: number, end: number): string {
  return String.fromCharCode(...bytes.subarray(start, end));
}

const K = new Uint32Array([
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
]);

const rotr = (value: number, bits: number) => (value >>> bits) | (value << (32 - bits));

/**
 * Incremental SHA-256 (FIPS 180-4). WebCrypto only digests one buffer, and a stream's packets
 * should not all sit in memory at once.
 */
export class Sha256 {
  private readonly state = new Uint32Array([
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
  ]);
  private readonly block = new Uint8Array(64);
  private readonly words = new Uint32Array(64);
  private buffered = 0;
  private length = 0;

  update(data: Uint8Array): this {
    this.length += data.length;
    let offset = 0;
    if (this.buffered) {
      offset = Math.min(64 - this.buffered, data.length);
      this.block.set(data.subarray(0, offset), this.buffered);
      this.buffered += offset;
      if (this.buffered < 64) return this;
      this.compress(this.block, 0);
      this.buffered = 0;
    }
    for (; offset + 64 <= data.length; offset += 64) this.compress(data, offset);
    this.block.set(data.subarray(offset));
    this.buffered = data.length - offset;
    return this;
  }

  /** Finishes the digest; the instance is spent afterwards. */
  hex(): string {
    const bits = this.length * 8;
    const padding = new Uint8Array((this.buffered < 56 ? 64 : 128) - this.buffered);
    padding[0] = 0x80;
    const view = new DataView(padding.buffer);
    view.setUint32(padding.length - 8, Math.floor(bits / 2 ** 32));
    view.setUint32(padding.length - 4, bits >>> 0);
    this.update(padding);
    return Array.from(this.state, (word) => word.toString(16).padStart(8, "0")).join("");
  }

  private compress(data: Uint8Array, offset: number) {
    const w = this.words;
    for (let i = 0; i < 16; i++) {
      const at = offset + i * 4;
      w[i] = (data[at] << 24) | (data[at + 1] << 16) | (data[at + 2] << 8) | data[at + 3];
    }
    for (let i = 16; i < 64; i++) {
      const s0 = rotr(w[i - 15], 7) ^ rotr(w[i - 15], 18) ^ (w[i - 15] >>> 3);
      const s1 = rotr(w[i - 2], 17) ^ rotr(w[i - 2], 19) ^ (w[i - 2] >>> 10);
      w[i] = w[i - 16] + s0 + w[i - 7] + s1;
    }
    let [a, b, c, d, e, f, g, h] = this.state;
    for (let i = 0; i < 64; i++) {
      const t1 = (h + (rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25)) + ((e & f) ^ (~e & g)) + K[i] + w[i]) | 0;
      const t2 = ((rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22)) + ((a & b) ^ (a & c) ^ (b & c))) | 0;
      h = g;
      g = f;
      f = e;
      e = (d + t1) | 0;
      d = c;
      c = b;
      b = a;
      a = (t1 + t2) | 0;
    }
    this.state[0] += a;
    this.state[1] += b;
    this.state[2] += c;
    this.state[3] += d;
    this.state[4] += e;
    this.state[5] += f;
    this.state[6] += g;
    this.state[7] += h;
  }
}
