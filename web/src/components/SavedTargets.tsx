import { useState } from "react";
import {
  deleteSavedTarget,
  listSavedTargets,
  saveTarget,
  type SavedTarget,
} from "../lib/saved-targets";

export function SavedTargets({
  current,
  disabled,
  onApply,
}: {
  current: { requirements: string; constraintsJson: string } | null;
  disabled: boolean;
  onApply: (target: SavedTarget) => void;
}) {
  const [targets, setTargets] = useState(listSavedTargets);
  const [name, setName] = useState("");
  const [editing, setEditing] = useState<SavedTarget | null>(null);
  const [removed, setRemoved] = useState<SavedTarget | null>(null);
  const [message, setMessage] = useState("");
  function save() {
    const content = editing ?? current;
    if (!content) return;
    try {
      saveTarget({ ...content, name });
      setTargets(listSavedTargets());
      setEditing(null);
      setName("");
      setMessage("Target saved.");
    } catch (error) {
      setMessage(
        error instanceof Error ? error.message : "Could not save target.",
      );
    }
  }
  return (
    <section className="saved-targets" aria-label="Saved targets">
      <p className="helper">
        Reuse your requirements. Files are never saved here.
      </p>
      {!targets.length ? (
        <p className="empty-copy">
          No saved targets yet. Choose a destination or enter requirements, then
          give the target a name.
        </p>
      ) : (
        <ul className="saved-list">
          {targets.map((target) => (
            <li key={target.id}>
              <button
                className="saved-chip"
                disabled={disabled}
                onClick={() => onApply(target)}
              >
                {target.name}
              </button>
              <button
                className="ft-btn ft-btn-text"
                disabled={disabled}
                aria-label={`Rename ${target.name}`}
                onClick={() => {
                  setEditing(target);
                  setName(target.name);
                }}
              >
                Rename
              </button>
              <button
                className="ft-btn ft-btn-text"
                disabled={disabled}
                aria-label={`Remove ${target.name}`}
                onClick={() => {
                  deleteSavedTarget(target.id);
                  setRemoved(target);
                  setTargets(listSavedTargets());
                  setMessage("Target removed.");
                }}
              >
                Remove
              </button>
            </li>
          ))}
        </ul>
      )}
      {current || editing ? (
        <form
          className="saved-form"
          onSubmit={(event) => {
            event.preventDefault();
            save();
          }}
        >
          <label htmlFor="target-name">
            {editing ? "Rename target" : "Save current requirements"}
          </label>
          <div className="saved-row">
            <input className="ft-field"
              id="target-name"
              value={name}
              disabled={disabled}
              onChange={(event) => setName(event.target.value)}
              placeholder="e.g. Application photo"
              required
              maxLength={80}
            />
            <button className="ft-btn ft-btn-secondary" disabled={disabled || !name.trim()}>
              Save target
            </button>
          </div>
          {editing ? (
            <button
              className="ft-btn ft-btn-text"
              type="button"
              onClick={() => {
                setEditing(null);
                setName("");
              }}
            >
              Cancel rename
            </button>
          ) : null}
        </form>
      ) : null}
      <p role="status" className="helper">
        {message}{" "}
        {removed ? (
          <button
            className="ft-btn ft-btn-text"
            disabled={disabled}
            onClick={() => {
              try {
                saveTarget(removed);
                setTargets(listSavedTargets());
                setRemoved(null);
                setMessage("Target restored.");
              } catch (error) {
                setMessage(error instanceof Error ? error.message : "Could not restore target.");
              }
            }}
          >
            Undo removal
          </button>
        ) : null}
      </p>
    </section>
  );
}
