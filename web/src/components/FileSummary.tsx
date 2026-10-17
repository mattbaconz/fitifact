import { formatBytes } from "../lib/constraints";
import { fileNameFromPath } from "../lib/desktop";
import { needsDesktopHandoff } from "../lib/docs-url";
import type { FitifactSession } from "../useFitifactSession";
import { RejectBlock } from "../useFitifactSession";
export function FileSummary({ session }: { session: FitifactSession }) {
  const {
    sourceUrl,
    sourceFile,
    mediaPath,
    inspectFacts,
    heicPreviewMissing,
    dragging,
    state,
    onDropKey,
    setDragging,
    onDrop,
    desktop,
    chooseDesktopFile,
    fileInputRef,
    accept,
    inspectFile,
    lastRejected,
  } = session;
  return (
    <section className="card file-card">
      <div className="file-card-main">
        {sourceUrl ? (
          <img
            className="file-thumb"
            src={sourceUrl}
            alt={`Preview of ${sourceFile?.name ?? "selected image"}`}
          />
        ) : null}
        <div className="file-chip file-row">
          <strong>
            {sourceFile?.name ?? (mediaPath ? fileNameFromPath(mediaPath) : "")}
          </strong>
          <span className="numeric">
            {inspectFacts ?? formatBytes(sourceFile?.size ?? 0)}
          </span>
          {heicPreviewMissing ? (
            <span>Preview unavailable for this phone photo.</span>
          ) : null}
        </div>
      </div>
      <div
        className={`drop-zone drop-zone-compact ${dragging ? "is-dragging" : ""} ${state === "processing" ? "is-disabled" : ""}`}
        aria-disabled={state === "processing"}
        aria-label="Replace file"
        tabIndex={0}
        onKeyDown={onDropKey}
        onDragOver={(event) => {
          event.preventDefault();
          if (state !== "processing") setDragging(true);
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={onDrop}
      >
        {desktop ? (
          <button
            className="ft-btn ft-btn-secondary button-label"
            type="button"
            disabled={state === "processing"}
            onClick={() => void chooseDesktopFile()}
          >
            Choose another file
          </button>
        ) : (
          <label className="ft-btn ft-btn-secondary button-label" htmlFor="image-file-replace">
            Choose another file
          </label>
        )}
        <input
          hidden={desktop}
          id="image-file-replace"
          ref={fileInputRef}
          className="visually-hidden"
          type="file"
          accept={accept}
          onChange={(event) => {
            const file = event.currentTarget.files?.item(0);
            if (file) void inspectFile(file);
            event.currentTarget.value = "";
          }}
          disabled={state === "processing"}
        />
      </div>
      {lastRejected ? (
        <RejectBlock
          name={lastRejected.name}
          message={lastRejected.message}
          offerDesktop={!desktop && needsDesktopHandoff(lastRejected.message)}
        />
      ) : null}
    </section>
  );
}
