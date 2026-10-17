type ClipboardItemClass = { supports?: (type: string) => boolean };

/** Copy and share appear only where this platform can hand over the exact validated file. */
export function fileActions(file: File | null): { copy: boolean; share: boolean } {
  if (!file) return { copy: false, share: false };
  const Item = (globalThis as { ClipboardItem?: ClipboardItemClass }).ClipboardItem;
  const canWrite = Boolean(Item) && typeof navigator.clipboard?.write === "function";
  // Without ClipboardItem.supports, PNG is the one image type every async clipboard accepts.
  const copy = canWrite && (Item?.supports ? Item.supports(file.type) : file.type === "image/png");
  return { copy, share: canShareFile(file) };
}

function canShareFile(file: File): boolean {
  try {
    return typeof navigator.canShare === "function" && navigator.canShare({ files: [file] });
  } catch {
    return false;
  }
}

export async function copyFile(file: File): Promise<void> {
  await navigator.clipboard.write([new ClipboardItem({ [file.type]: file })]);
}

export async function shareFile(file: File): Promise<void> {
  await navigator.share({ files: [file], title: file.name });
}
