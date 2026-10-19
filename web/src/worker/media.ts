/**
 * Browser video provider: Mediabunny probes, copies streams into MP4 or re-encodes them through
 * WebCodecs as the engine instructs, and re-reads the result. Loaded on demand by the worker, so
 * photo users never download it. Planning, the bitrate ladder, and validation stay in the Rust
 * engine; encoders start only for a plan that needs one.
 */
import {
  BlobSource,
  BufferTarget,
  Conversion,
  EncodedPacketSink,
  Input,
  MATROSKA,
  MP4,
  Mp4OutputFormat,
  NullTarget,
  Output,
  QTFF,
  Quality,
  VideoSample,
  VideoSampleSource,
  WEBM,
  canEncodeAudio,
  canEncodeVideo,
  type InputAudioTrack,
  type InputTrack,
  type InputVideoTrack,
} from "mediabunny";
import type { MeasureRequest, Measured, TranscodeExecution } from "./protocol";
import { CensusFailure, Sha256, trackCensus } from "./media-census";

export const MEDIABUNNY_VERSION = "1.61.1";

// ponytail: BufferTarget holds the whole output in memory; StreamTarget plus a save picker
// would lift this ceiling.
export const MAX_REMUX_BYTES = 1024 * 1024 * 1024;

export class MediaFailure extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message);
  }
}

const UNREADABLE =
  "The web app couldn't read this video. It may be damaged, or only the Fitifact desktop app can open it.";

// The containers the census accepts; other demuxers stay out of the bundle.
const FORMATS = [MP4, QTFF, MATROSKA, WEBM];

function open(file: Blob) {
  return new Input({ formats: FORMATS, source: new BlobSource(file) });
}

/** The fitifact.browser-probe/v1 facts the engine turns into an artifact. */
export async function probe(file: File) {
  let census;
  try {
    census = await trackCensus(file);
  } catch (error) {
    if (error instanceof CensusFailure) throw new MediaFailure("INSPECTION_UNSUPPORTED", UNREADABLE);
    throw error;
  }
  const input = open(file);
  try {
    const videos = await input.getVideoTracks();
    const audios = await input.getAudioTracks();
    let video = 0;
    let audio = 0;
    const tracks = [];
    for (const type of census.tracks) {
      if (type === "video") tracks.push(await videoFacts(videos[video++]));
      else if (type === "audio") tracks.push(await audioFacts(audios[audio++]));
      else tracks.push({ type });
    }
    if (video < videos.length || audio < audios.length) {
      throw new MediaFailure("INSPECTION_UNSUPPORTED", UNREADABLE);
    }
    return {
      schema: "fitifact.browser-probe/v1",
      provider_version: MEDIABUNNY_VERSION,
      name: file.name,
      byte_length: file.size,
      container: census.container,
      major_brand: census.majorBrand,
      duration_ms: Math.round((await input.computeDuration()) * 1000),
      tracks,
    };
  } catch (error) {
    if (error instanceof MediaFailure) throw error;
    throw new MediaFailure("INSPECTION_UNSUPPORTED", UNREADABLE);
  } finally {
    input.dispose();
  }
}

async function videoFacts(track: InputVideoTrack | undefined) {
  if (!track) return { type: "video" };
  const codec = await track.getCodec();
  const config = codec === "avc" ? await track.getDecoderConfig() : null;
  const color = await track.getColorSpace();
  return {
    type: "video",
    codec,
    width: await track.getCodedWidth(),
    height: await track.getCodedHeight(),
    avcc: config?.description ? Array.from(bytesOf(config.description)) : null,
    color: {
      primaries: color.primaries ?? null,
      transfer: color.transfer ?? null,
      matrix: color.matrix ?? null,
      full_range: color.fullRange ?? null,
    },
  };
}

async function audioFacts(track: InputAudioTrack | undefined) {
  if (!track) return { type: "audio" };
  return {
    type: "audio",
    codec: await track.getCodec(),
    channels: await track.getNumberOfChannels(),
    sample_rate: await track.getSampleRate(),
  };
}

function bytesOf(source: AllowSharedBufferSource): Uint8Array {
  return ArrayBuffer.isView(source)
    ? new Uint8Array(source.buffer, source.byteOffset, source.byteLength)
    : new Uint8Array(source);
}

/** Copies every stream into MP4 without re-encoding, or refuses if any stream would be lost. */
export async function remuxToMp4(file: Blob, onProgress: (fraction: number) => void): Promise<ArrayBuffer> {
  if (file.size > MAX_REMUX_BYTES) {
    throw new MediaFailure(
      "EXECUTION_LIMIT",
      "This video is too large to repackage in the browser. Open it in the Fitifact desktop app.",
    );
  }
  const input = open(file);
  try {
    const target = new BufferTarget();
    const conversion = await Conversion.init({
      input,
      output: new Output({ format: new Mp4OutputFormat({ fastStart: "in-memory" }), target }),
      copy: { mode: "forced" },
      showWarnings: false,
    });
    if (!conversion.isValid || conversion.discardedTracks.length) {
      throw new MediaFailure(
        "NO_VALID_PLAN",
        "A stream in this video can't be copied into MP4 as is. Open it in the Fitifact desktop app.",
      );
    }
    conversion.onProgress = (fraction) => onProgress(fraction);
    await conversion.execute();
    if (!target.buffer) throw new MediaFailure("EXECUTION_FAILED", "The MP4 was not finished.");
    return target.buffer;
  } finally {
    input.dispose();
  }
}

/**
 * What the engine asked to know before it judges an encode here: how the source is shown,
 * whether this browser decodes its streams, and the H.264 configuration its encoder writes.
 */
export async function measure(file: Blob, request: MeasureRequest): Promise<Measured> {
  const input = open(file);
  try {
    const video = (await input.getVideoTracks())[0];
    const audio = (await input.getAudioTracks())[0];
    const encode = request.audio;
    return {
      geometry: video ? await displayGeometry(video) : null,
      video_decode: video ? await supported(() => video.canDecode()) : false,
      h264_sample: await h264Sample(request.video),
      audio_decode: encode && audio ? await supported(() => audio.canDecode()) : false,
      aac_encode: encode
        ? await supported(() =>
            canEncodeAudio("aac", {
              numberOfChannels: encode.channels,
              sampleRate: encode.sample_rate ?? undefined,
              bitrate: encode.bitrate,
            }),
          )
        : false,
    };
  } finally {
    input.dispose();
  }
}

async function supported(check: () => Promise<boolean>): Promise<boolean> {
  try {
    return await check();
  } catch {
    return false;
  }
}

/** The decoder configuration this browser's H.264 encoder writes for one BT.709 frame. */
async function h264Sample({ width, height, bitrate }: MeasureRequest["video"]): Promise<number[] | null> {
  if (!(await supported(() => canEncodeVideo("avc", { width, height, bitrate })))) return null;
  let description: Uint8Array | null = null;
  const source = new VideoSampleSource({
    codec: "avc",
    quality: new Quality({ bitrate }),
    onEncodedPacket: (_, meta) => {
      const config = meta?.decoderConfig?.description;
      if (config && !description) description = bytesOf(config).slice();
    },
  });
  const output = new Output({ format: new Mp4OutputFormat(), target: new NullTarget() });
  output.addVideoTrack(source);
  const sample = bt709Sample(new Uint8Array((width * height * 3) / 2).fill(128), width, height, 0, 1 / 30);
  let timer = 0;
  // One small frame takes well under a second; an encoder that never answers is not usable.
  const stalled = new Promise<"stalled">((resolve) => {
    timer = self.setTimeout(() => resolve("stalled"), SAMPLE_TIMEOUT_MS);
  });
  try {
    const encoded = (async () => {
      await output.start();
      await source.add(sample);
      await output.finalize();
    })();
    encoded.catch(() => undefined); // a stalled encode may still fail after the race
    if ((await Promise.race([encoded, stalled])) === "stalled") {
      void output.cancel();
      return null;
    }
  } catch {
    return null;
  } finally {
    self.clearTimeout(timer);
    sample.close();
  }
  return description ? Array.from(description) : null;
}

const SAMPLE_TIMEOUT_MS = 15_000;

const BT709: VideoColorSpaceInit = { primaries: "bt709", transfer: "bt709", matrix: "bt709", fullRange: false };

function bt709Sample(i420: Uint8Array, width: number, height: number, timestamp: number, duration: number) {
  return new VideoSample(i420, {
    format: "I420",
    codedWidth: width,
    codedHeight: height,
    timestamp,
    duration,
    colorSpace: BT709,
  });
}

/**
 * Runs the engine's H.264 ladder: one full encode per bitrate until the MP4 fits the byte limit.
 * The engine ends the ladder at its quality floor, so running out means the target is too small.
 */
export async function transcodeToMp4(
  file: Blob,
  execution: TranscodeExecution,
  onProgress: (fraction: number, attempt: number) => void,
): Promise<ArrayBuffer> {
  if (file.size > MAX_REMUX_BYTES) {
    throw new MediaFailure(
      "EXECUTION_LIMIT",
      "This video is too large to re-encode in the browser. Open it in the Fitifact desktop app.",
    );
  }
  for (const [attempt, bitrate] of execution.video_bitrates.entries()) {
    const output = await encodeOnce(file, execution, bitrate, (fraction) => onProgress(fraction, attempt));
    if (execution.max_bytes === null || output.byteLength <= execution.max_bytes) return output;
  }
  throw new MediaFailure(
    "EXECUTION_LIMIT",
    "Even at the lowest quality Fitifact allows, this video doesn't fit the size limit.",
  );
}

async function encodeOnce(
  file: Blob,
  execution: TranscodeExecution,
  bitrate: number,
  onProgress: (fraction: number) => void,
): Promise<ArrayBuffer> {
  const { width, height, audio } = execution;
  const input = open(file);
  try {
    const target = new BufferTarget();
    const conversion = await Conversion.init({
      input,
      output: new Output({ format: new Mp4OutputFormat({ fastStart: "in-memory" }), target }),
      video: {
        codec: "avc",
        width,
        height,
        fit: "fill",
        quality: new Quality({ bitrate }),
        forceTranscode: true,
        process: (sample) => toBt709(sample, width, height),
      },
      audio:
        audio === "copy"
          ? undefined
          : {
              codec: "aac",
              numberOfChannels: audio.channels,
              quality: new Quality({ bitrate: audio.bitrate }),
              forceTranscode: true,
            },
      showWarnings: false,
    });
    if (!conversion.isValid || conversion.discardedTracks.length) {
      throw new MediaFailure(
        "NO_VALID_PLAN",
        "This browser can't re-encode every stream in this video. Open it in the Fitifact desktop app.",
      );
    }
    conversion.onProgress = (fraction) => onProgress(fraction);
    await conversion.execute();
    if (!target.buffer) throw new MediaFailure("EXECUTION_FAILED", "The MP4 was not finished.");
    return target.buffer;
  } finally {
    input.dispose();
  }
}

let canvas: OffscreenCanvasRenderingContext2D | null = null;

/**
 * Redraws a frame as limited-range BT.709 4:2:0, the picture the engine plans and FFmpeg writes.
 * Browsers turn canvas pixels into video with other matrices and tags, so the worker converts.
 */
// ponytail: per-pixel JavaScript; move to a WebGL pass if 1080p encodes prove too slow.
function toBt709(sample: VideoSample, width: number, height: number): VideoSample {
  // WebCodecs names these HDR curves; the DOM typings do not list them yet.
  const transfer: string | null = sample.colorSpace.transfer;
  if (transfer === "pq" || transfer === "hlg") {
    throw new MediaFailure(
      "NO_VALID_PLAN",
      "This video is HDR, and the web app can't convert HDR to standard video. Open it in the Fitifact desktop app.",
    );
  }
  if (!canvas || canvas.canvas.width !== width || canvas.canvas.height !== height) {
    canvas = new OffscreenCanvas(width, height).getContext("2d", { willReadFrequently: true });
    if (!canvas) throw new MediaFailure("EXECUTION_FAILED", "This browser can't draw video frames.");
  }
  sample.draw(canvas, 0, 0, width, height);
  return bt709Sample(
    rgbaToI420(canvas.getImageData(0, 0, width, height).data, width, height),
    width,
    height,
    sample.timestamp,
    sample.duration,
  );
}

const QUAD = [
  [0, 0],
  [1, 0],
  [0, 1],
  [1, 1],
];

/** Limited-range BT.709 Y'CbCr of even-sized RGBA, with chroma averaged over each 2x2 block. */
export function rgbaToI420(rgba: Uint8ClampedArray, width: number, height: number): Uint8Array {
  const out = new Uint8Array((width * height * 3) / 2);
  const chromaWidth = width / 2;
  const cb = width * height;
  const cr = cb + (chromaWidth * height) / 2;
  for (let y = 0; y < height; y += 2) {
    for (let x = 0; x < width; x += 2) {
      let r = 0;
      let g = 0;
      let b = 0;
      for (const [dx, dy] of QUAD) {
        const pixel = (y + dy) * width + x + dx;
        const [pr, pg, pb] = [rgba[pixel * 4], rgba[pixel * 4 + 1], rgba[pixel * 4 + 2]];
        out[pixel] = Math.round(16 + 0.1826 * pr + 0.6142 * pg + 0.062 * pb);
        r += pr;
        g += pg;
        b += pb;
      }
      const chroma = (y / 2) * chromaWidth + x / 2;
      out[cb + chroma] = Math.round(128 + (-0.1006 * r - 0.3386 * g + 0.4392 * b) / 4);
      out[cr + chroma] = Math.round(128 + (0.4392 * r - 0.3989 * g - 0.0403 * b) / 4);
    }
  }
  return out;
}

/** SHA-256 of each stream's packet payloads in decode order, like FFmpeg's streamhash muxer. */
export async function streamFacts(file: Blob) {
  const input = open(file);
  try {
    const video = (await input.getVideoTracks())[0];
    const audio = (await input.getAudioTracks())[0];
    return {
      hashes: {
        algorithm: "sha256",
        video: video ? await hashPackets(video) : "",
        audio: audio ? await hashPackets(audio) : null,
      },
      geometry: video ? await displayGeometry(video) : null,
    };
  } finally {
    input.dispose();
  }
}

/** How a player shows the picture: display size (pixel aspect applied), rotation, and mirroring. */
async function displayGeometry(track: InputVideoTrack) {
  return {
    display_width: await track.getDisplayWidth(),
    display_height: await track.getDisplayHeight(),
    rotation: await track.getRotation(),
    flip: await track.getFlip(),
  };
}

async function hashPackets(track: InputTrack): Promise<string> {
  const hash = new Sha256();
  for await (const packet of new EncodedPacketSink(track).packets()) hash.update(packet.data);
  return hash.hex();
}
