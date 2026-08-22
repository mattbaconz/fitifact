import type { CropRectangle } from "../types";

export function cropForAspect(
  sourceWidth: number,
  sourceHeight: number,
  targetWidth: number,
  targetHeight: number,
  positionPercent: number,
): CropRectangle {
  const position = Math.min(1, Math.max(0, positionPercent / 100));
  const sourceAspect = sourceWidth / sourceHeight;
  const targetAspect = targetWidth / targetHeight;
  if (sourceAspect > targetAspect) {
    const width = targetAspect / sourceAspect;
    return { x: (1 - width) * position, y: 0, width, height: 1 };
  }
  const height = sourceAspect / targetAspect;
  return { x: 0, y: (1 - height) * position, width: 1, height };
}

export function cropAxis(sourceWidth: number, sourceHeight: number, targetWidth: number, targetHeight: number) {
  return sourceWidth / sourceHeight > targetWidth / targetHeight ? "horizontal" : "vertical";
}
