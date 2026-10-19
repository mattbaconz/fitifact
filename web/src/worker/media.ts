/**
 * Browser video provider: Mediabunny probes, copies streams into MP4, and re-reads the result.
 * Loaded on demand by the worker, so photo users never download it. It never decodes or
 * encodes; planning and validation stay in the Rust engine.
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
  Output,
  QTFF,
  WEBM,
  type InputAudioTrack,
  type InputTrack,
  type InputVideoTrack,
} from "mediabunny";
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
      geometry: video ? await displayGeometry(video) : "",
    };
  } finally {
    input.dispose();
  }
}

/** How a player shows the picture: display size (pixel aspect applied), rotation, and mirroring. */
async function displayGeometry(track: InputVideoTrack): Promise<string> {
  const facts = [
    await track.getDisplayWidth(),
    await track.getDisplayHeight(),
    await track.getRotation(),
    await track.getFlip(),
  ];
  return facts.join(" ");
}

async function hashPackets(track: InputTrack): Promise<string> {
  const hash = new Sha256();
  for await (const packet of new EncodedPacketSink(track).packets()) hash.update(packet.data);
  return hash.hex();
}
