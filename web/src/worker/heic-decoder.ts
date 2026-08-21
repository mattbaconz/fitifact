// Optional isolated adapter for libheif-js 1.19.8. See ../../public/THIRD_PARTY_NOTICES.md.
export interface DecodedHeic {
  rgba: Uint8Array;
  width: number;
  height: number;
}

interface HeifImage {
  get_width(): number;
  get_height(): number;
  display(
    target: { data: Uint8ClampedArray; width: number; height: number },
    callback: (value: unknown) => void,
  ): void;
  free?: () => void;
}

interface LibHeif {
  HeifDecoder: new () => { decode(bytes: Uint8Array): HeifImage[] };
}

export class HeicDecodeFailure extends Error {
  constructor(
    readonly code: "INSPECTION_LIMIT" | "INSPECTION_UNSUPPORTED" | "EXECUTION_FAILED",
    message: string,
  ) {
    super(message);
  }
}

export function decodedRgbaLength(width: number, height: number, maxPixels: number): number {
  const pixels = width * height;
  if (!Number.isSafeInteger(pixels) || width < 1 || height < 1 || pixels > maxPixels) {
    throw new HeicDecodeFailure(
      "INSPECTION_LIMIT",
      `This HEIC exceeds the ${maxPixels.toLocaleString("en-US")} pixel local processing limit.`,
    );
  }
  return pixels * 4;
}

export async function decodeSingleHeic(
  bytes: Uint8Array,
  maxDecodedPixels: number,
): Promise<DecodedHeic> {
  const imported = await import("libheif-js/wasm-bundle");
  const libheif = (imported.default ?? imported) as LibHeif;
  if (typeof libheif.HeifDecoder !== "function") {
    throw new HeicDecodeFailure(
      "EXECUTION_FAILED",
      "The approved HEIC decoder build could not be initialized.",
    );
  }
  const images = new libheif.HeifDecoder().decode(bytes);
  if (images.length !== 1) {
    for (const image of images) image.free?.();
    throw new HeicDecodeFailure(
      "INSPECTION_UNSUPPORTED",
      "Animated or multi-image HEIC files are not supported.",
    );
  }
  const image = images[0];
  const width = image.get_width();
  const height = image.get_height();
  let rgbaLength: number;
  try {
    rgbaLength = decodedRgbaLength(width, height, maxDecodedPixels);
  } catch (error) {
    image.free?.();
    throw error;
  }
  const data = new Uint8ClampedArray(rgbaLength);
  try {
    await new Promise<void>((resolve, reject) => {
      image.display({ data, width, height }, (displayed) => {
        if (displayed) resolve();
        else {
          reject(
            new HeicDecodeFailure(
              "EXECUTION_FAILED",
              "The HEIC decoder could not produce pixels.",
            ),
          );
        }
      });
    });
  } finally {
    image.free?.();
  }
  return { rgba: new Uint8Array(data.buffer), width, height };
}
