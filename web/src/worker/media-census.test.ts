import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import packageJson from "../../package.json";
import { CensusFailure, Sha256, trackCensus } from "./media-census";
import { MEDIABUNNY_VERSION, rgbaToI420 } from "./media";

async function fixture(name: string) {
  const path = fileURLToPath(new URL(`../../../fixtures/media/${name}`, import.meta.url));
  return new Blob([await readFile(path)]);
}

/** An EBML element: its ID bytes, an 8-byte size (or unknown), then the body. */
function el(id: number[], ...children: Uint8Array[]): Uint8Array {
  const body = Buffer.concat(children);
  const size = Buffer.alloc(8);
  size.writeBigUInt64BE(BigInt(body.length));
  size[0] = 0x01;
  return Buffer.concat([Buffer.from(id), size, body]);
}

function openEnded(id: number[], ...children: Uint8Array[]): Uint8Array {
  return Buffer.concat([Buffer.from(id), Buffer.from([0x01, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]), ...children]);
}

const EBML = el([0x1a, 0x45, 0xdf, 0xa3], el([0x42, 0x82], Buffer.from("matroska")));
const SEGMENT = [0x18, 0x53, 0x80, 0x67];
const TRACKS = [0x16, 0x54, 0xae, 0x6b];
const CLUSTER = [0x1f, 0x43, 0xb6, 0x75];
const track = (type: number) => el([0xae], el([0x83], Uint8Array.of(type)));
const attachments = (count: number) =>
  el([0x19, 0x41, 0xa4, 0x69], ...Array.from({ length: count }, () => el([0x61, 0xa7])));

describe("track census", () => {
  it("lists every ISOBMFF track by handler, in stream order", async () => {
    await expect(trackCensus(await fixture("compatible-h264-aac.mp4"))).resolves.toEqual({
      container: "isobmff",
      majorBrand: "isom",
      tracks: ["video", "audio"],
    });
    await expect(trackCensus(await fixture("remux-h264-aac.mov"))).resolves.toMatchObject({
      majorBrand: "qt  ",
      tracks: ["video", "audio"],
    });
    await expect(trackCensus(await fixture("unsupported-extra-video.mp4"))).resolves.toMatchObject({
      tracks: ["video", "audio", "video"],
    });
  });

  it("reads FFmpeg-written Matroska and WebM files", async () => {
    for (const name of ["remux-h264-aac.mkv", "transcode-vp9-opus.webm"]) {
      await expect(trackCensus(await fixture(name))).resolves.toEqual({
        container: "matroska",
        majorBrand: null,
        tracks: ["video", "audio"],
      });
    }
  });

  it("keeps subtitles and attachments visible so the planner can refuse them", async () => {
    const file = Buffer.concat([
      EBML,
      el(SEGMENT, el(TRACKS, track(1), track(2), track(0x11)), attachments(2), el(CLUSTER)),
    ]);
    await expect(trackCensus(new Blob([file]))).resolves.toMatchObject({
      tracks: ["video", "audio", "subtitle", "attachment", "attachment"],
    });
  });

  it("accepts a live recording's open-ended clusters only after the track list", async () => {
    const live = Buffer.concat([EBML, openEnded(SEGMENT, el(TRACKS, track(1)), openEnded(CLUSTER))]);
    await expect(trackCensus(new Blob([live]))).resolves.toMatchObject({ tracks: ["video"] });
    const early = Buffer.concat([EBML, openEnded(SEGMENT, openEnded(CLUSTER, el(TRACKS, track(1))))]);
    await expect(trackCensus(new Blob([early]))).rejects.toBeInstanceOf(CensusFailure);
  });

  it("refuses damaged or foreign files instead of guessing", async () => {
    await expect(trackCensus(await fixture("corrupt-truncated.mp4"))).rejects.toBeInstanceOf(CensusFailure);
    await expect(trackCensus(new Blob([Buffer.from("%PDF-1.7 not a video")]))).rejects.toBeInstanceOf(
      CensusFailure,
    );
    const noMoov = Buffer.alloc(16);
    noMoov.writeUInt32BE(16, 0);
    noMoov.write("ftypisom", 4, "ascii");
    await expect(trackCensus(new Blob([noMoov]))).rejects.toBeInstanceOf(CensusFailure);
  });
});

describe("incremental SHA-256", () => {
  it("matches the FIPS 180-4 vectors", () => {
    expect(new Sha256().hex()).toBe("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    expect(new Sha256().update(Buffer.from("abc")).hex()).toBe(
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );
  });

  it("matches node:crypto across padding boundaries and uneven chunks", () => {
    const data = Uint8Array.from({ length: 1_000 }, (_, index) => (index * 31 + 7) & 0xff);
    for (let length = 0; length <= 200; length++) {
      const slice = data.subarray(0, length);
      expect(new Sha256().update(slice).hex()).toBe(createHash("sha256").update(slice).digest("hex"));
    }
    const hash = new Sha256();
    let offset = 0;
    for (const size of [1, 63, 64, 65, 7, 0, 128, 300]) {
      hash.update(data.subarray(offset, offset + size));
      offset += size;
    }
    hash.update(data.subarray(offset));
    expect(hash.hex()).toBe(createHash("sha256").update(data).digest("hex"));
  });
});

it("reports the pinned Mediabunny version as the probe provider", () => {
  expect(packageJson.dependencies.mediabunny).toBe(MEDIABUNNY_VERSION);
});

it("redraws frames as limited-range BT.709 4:2:0, one chroma sample per 2x2 block", () => {
  const rgba = (...pixels: number[][]) => Uint8ClampedArray.from(pixels.flatMap((rgb) => [...rgb, 255]));
  const [red, blue, white] = [[255, 0, 0], [0, 0, 255], [255, 255, 255]];
  // A 4x2 frame, red on the left and blue on the right: Y plane, then Cb, then Cr.
  expect([...rgbaToI420(rgba(red, red, blue, blue, red, red, blue, blue), 4, 2)]).toEqual([
    63, 63, 32, 32, 63, 63, 32, 32, 102, 240, 240, 118,
  ]);
  expect([...rgbaToI420(rgba(white, white, white, white), 2, 2)]).toEqual([235, 235, 235, 235, 128, 128]);
});
