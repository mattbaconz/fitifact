export class EncodedResourceLimit extends Error {}

export interface FileLike {
  readonly size: number;
  arrayBuffer(): Promise<ArrayBuffer>;
}

export function assertEncodedLength(length: number, maxEncodedBytes: number) {
  if (!Number.isSafeInteger(length) || length < 0 || length > maxEncodedBytes) {
    throw new EncodedResourceLimit(`This file exceeds the ${maxEncodedBytes} byte local input limit.`);
  }
}

export async function readFileWithinLimit(file: FileLike, maxEncodedBytes: number) {
  assertEncodedLength(file.size, maxEncodedBytes);
  const buffer = await file.arrayBuffer();
  assertEncodedLength(buffer.byteLength, maxEncodedBytes);
  return buffer;
}

export function enterWasmWithEncodedLimit<T>(
  encodedLength: number,
  maxEncodedBytes: number,
  operation: () => T,
): T {
  assertEncodedLength(encodedLength, maxEncodedBytes);
  return operation();
}
