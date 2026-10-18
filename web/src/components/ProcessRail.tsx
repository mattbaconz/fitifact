import type { ProductState } from "../types";

export function ProcessRail({
  state,
  stage,
  hasFile,
}: {
  state: ProductState;
  stage?: string;
  hasFile: boolean;
}) {
  const complete = state === "compatible" || state === "adapted";
  const ready = state === "planned" || state === "crop_approval_required";
  const working = state === "processing";
  const phase = /validat|check.*output/i.test(stage ?? "")
    ? 3
    : /applying|adapt|encod|transform/i.test(stage ?? "")
      ? 2
      : /plan|requirements|target/i.test(stage ?? "")
        ? 1
        : /inspect|decod|load.*engine/i.test(stage ?? "")
          ? 0
          : hasFile
            ? 1
            : 0;
  const labels = ["Inspect", "Plan", "Adapt", "Validate"];
  return (
    <ol className="process-rail" aria-label="File processing stages">
      {labels.map((label, index) => {
        const skipped = state === "compatible" && index === 2;
        const done =
          complete ||
          (hasFile && index === 0 && (!working || phase > 0)) ||
          ((ready || (working && phase > 1)) && index === 1);
        const current =
          !complete &&
          index === (working ? phase : ready ? 2 : hasFile ? 1 : 0);
        return (
          <li
            key={label}
            className={done ? "is-done" : current ? "is-current" : ""}
            aria-current={current ? "step" : undefined}
          >
            <span className="stage-symbol" aria-hidden="true">
              {skipped ? "−" : done ? "✓" : index + 1}
            </span>
            <span>
              {label}
              {skipped ? <small>Skipped</small> : null}
            </span>
          </li>
        );
      })}
    </ol>
  );
}
