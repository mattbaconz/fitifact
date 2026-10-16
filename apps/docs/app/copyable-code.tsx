"use client";
import { useRef, useState, type ComponentProps } from "react";

export function CopyableCode(props: ComponentProps<"pre">) {
  const ref = useRef<HTMLPreElement>(null);
  const [feedback, setFeedback] = useState("");
  return (
    <div className="fitifact-code">
      <button
        type="button"
        onClick={async () => {
          try {
            await navigator.clipboard.writeText(ref.current?.textContent ?? "");
            setFeedback("Copied.");
          } catch {
            setFeedback(
              "Copy unavailable. Select the code and copy it manually.",
            );
          }
        }}
      >
        Copy code
      </button>
      <pre {...props} ref={ref} />
      <span role="status">{feedback}</span>
    </div>
  );
}
