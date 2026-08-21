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

const MAX_PIXELS = 24_000_000;

export async function decodeSingleHeic(bytes: Uint8Array): Promise<DecodedHeic> {
  const imported = await import("libheif-js/wasm-bundle");
  const libheif = (imported.default ?? imported) as LibHeif;
  if (typeof libheif.HeifDecoder !== "function") {
    throw new Error("The approved HEIC decoder build could not be initialized.");
  }
  const images = new libheif.HeifDecoder().decode(bytes);
  if (images.length !== 1) {
    for (const image of images) image.free?.();
    throw new Error("Animated or multi-image HEIC files are not supported.");
  }
  const image = images[0];
  const width = image.get_width();
  const height = image.get_height();
  const pixels = width * height;
  if (!Number.isSafeInteger(pixels) || width < 1 || height < 1 || pixels > MAX_PIXELS) {
    image.free?.();
    throw new Error("This HEIC exceeds the 24 megapixel local processing limit.");
  }
  const data = new Uint8ClampedArray(pixels * 4);
  try {
    await new Promise<void>((resolve, reject) => {
      image.display({ data, width, height }, (displayed) => {
        if (displayed) resolve();
        else reject(new Error("The HEIC decoder could not produce pixels."));
      });
    });
  } finally {
    image.free?.();
  }
  return { rgba: new Uint8Array(data.buffer), width, height };
}
