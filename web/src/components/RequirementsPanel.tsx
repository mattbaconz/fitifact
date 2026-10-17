import { ProfileProvenance } from "./ProfileProvenance";
import { SavedTargets } from "../components/SavedTargets";
import {
  familyForProfile,
  profileForFamily,
  profilePlainSummary,
} from "../lib/destinations";
import { summarizeTarget } from "../lib/target-summary";
import type { FitifactSession } from "../useFitifactSession";
import { CHIP_ICONS } from "../useFitifactSession";
export function RequirementsPanel({
  session,
  changeRequirements,
}: {
  session: FitifactSession;
  changeRequirements: () => void;
}) {
  const {
    targetMode,
    setTargetMode,
    chips,
    activeProfile,
    doctorUnhealthy,
    fileKind,
    setup,
    state,
    applyDestination,
    usingCopy,
    setSidebarOpen,
    confirmedConstraintsJson,
    requirements,
    applySaved,
    onRequirementsPaste,
    editRequirements,
    understood,
    leftover,
    parsing,
    target,
    setEditingTarget,
    editingTarget,
    activeFamily,
    formatOptions,
    editTarget,
    targetDirty,
    replan,
    targetError,
    desktopTarget,
  } = session;
  return (
    <section
      className="card destination-card"
      aria-labelledby="destination-title"
    >
      <h2 id="destination-title">Where does it need to work?</h2>
      <div
        className="target-tabs"
        role="group"
        aria-label="Choose requirements"
      >
        <button
          className={targetMode === "destinations" ? "is-active" : ""}
          aria-pressed={targetMode === "destinations"}
          onClick={() => setTargetMode("destinations")}
        >
          Destinations
        </button>
        <button
          className={targetMode === "saved" ? "is-active" : ""}
          aria-pressed={targetMode === "saved"}
          onClick={() => setTargetMode("saved")}
        >
          Saved targets
        </button>
        <button
          className={targetMode === "custom" ? "is-active" : ""}
          aria-pressed={targetMode === "custom"}
          onClick={changeRequirements}
        >
          Custom
        </button>
      </div>
      {targetMode === "destinations" ? (
        <>
          <div className="destination-chips">
            {chips.map((chip) => {
              const Icon = CHIP_ICONS[chip.family];
              const selected =
                familyForProfile(activeProfile ?? "") === chip.family;
              const mediaLocked =
                doctorUnhealthy &&
                (fileKind === "video" || fileKind === "audio");
              const toolsLocked =
                (chip.videoOnly && doctorUnhealthy) || mediaLocked;
              const acceptsFile =
                !fileKind ||
                Boolean(
                  profileForFamily(chip.family, setup.discordCap, fileKind),
                );
              const chipLocked = toolsLocked || !acceptsFile;
              return (
                <button
                  key={chip.family}
                  type="button"
                  className={`destination-chip${selected ? " is-selected" : ""}`}
                  disabled={state === "processing" || chipLocked}
                  aria-pressed={selected}
                  onClick={() => void applyDestination(chip.family)}
                >
                  <span className="destination-chip-label">
                    <Icon />
                    {chip.label}
                  </span>
                  <span className="destination-chip-sub">
                    {!acceptsFile
                      ? "Not for this file"
                      : toolsLocked
                        ? "Video tools unavailable"
                        : chip.subtitle}
                  </span>
                </button>
              );
            })}
          </div>
          {usingCopy ? <p className="empty-copy">{usingCopy}</p> : null}

          <p className="helper">
            Limits vary by account.{" "}
            <button
              className="text-button inline-button"
              onClick={() => setSidebarOpen(true)}
            >
              Set your Discord cap
            </button>
          </p>
        </>
      ) : targetMode === "saved" ? (
        <SavedTargets
          current={
            confirmedConstraintsJson
              ? { requirements, constraintsJson: confirmedConstraintsJson }
              : null
          }
          disabled={state === "processing"}
          onApply={(saved) => void applySaved(saved)}
        />
      ) : (
        <section className="override-card" aria-labelledby="requirements-title">
          <h3 id="requirements-title">Use your own requirements</h3>
          <label htmlFor="requirements">
            Rejection message or requirements
          </label>
          <textarea
            id="requirements"
            rows={4}
            value={requirements}
            placeholder="Or paste the exact rejection (size, format, dimensions)."
            onPaste={onRequirementsPaste}
            onChange={(event) => editRequirements(event.target.value)}
            disabled={state === "processing"}
          />
          {understood ? (
            <div className="notice" role="note">
              {understood}
            </div>
          ) : null}
          {leftover ? <p className="leftover-copy">{leftover}</p> : null}

          {parsing ? (
            <p className="helper" role="status">
              Reading requirements…
            </p>
          ) : null}
          <p className="helper">
            For example: JPEG, max 2 MB, exactly 600 × 600. You can also paste
            an upload rejection.
          </p>
        </section>
      )}
      {target ? (
        <div className="confirmed-target">
          <div className="target-heading">
            <h3>Current requirements</h3>
            <button
              className="text-button"
              onClick={() => setEditingTarget((open) => !open)}
              disabled={state === "processing"}
            >
              {editingTarget ? "Hide editor" : "Edit requirements"}
            </button>
          </div>
          <p className="target-summary">{summarizeTarget(target)}</p>
          <p className="helper">
            {activeFamily
              ? "From a destination profile. Verify the limits for your account."
              : "From the requirements you supplied."}
          </p>
          {activeFamily ? <ProfileProvenance profileId={activeProfile} /> : null}
          {editingTarget ? (
            <div className="target-form">
              <fieldset
                className="format-options"
                disabled={state === "processing"}
              >
                <legend>Allowed formats</legend>
                {formatOptions.map((format) => (
                  <label key={format}>
                    <input
                      type="checkbox"
                      checked={target.formats.includes(format)}
                      onChange={(event) =>
                        editTarget((current) => ({
                          ...current,
                          formats: event.target.checked
                            ? [...current.formats, format]
                            : current.formats.filter((item) => item !== format),
                        }))
                      }
                    />
                    {format === "jpeg"
                      ? "JPEG"
                      : format === "png"
                        ? "PNG"
                        : "WebP"}
                  </label>
                ))}
              </fieldset>
              <label>
                Maximum size
                <input
                  aria-describedby="size-units-help"
                  value={target.maxBytes}
                  disabled={state === "processing"}
                  onChange={(event) =>
                    editTarget((current) => ({
                      ...current,
                      maxBytes: event.target.value,
                    }))
                  }
                  placeholder="No limit, or e.g. 2 MB"
                />
              </label>
              <label>
                Minimum size
                <input
                  aria-describedby="size-units-help"
                  value={target.minBytes}
                  disabled={state === "processing"}
                  onChange={(event) =>
                    editTarget((current) => ({
                      ...current,
                      minBytes: event.target.value,
                    }))
                  }
                  placeholder="None, or e.g. 20 KB"
                />
              </label>
              <p id="size-units-help" className="helper">
                MB = 1,000,000 bytes. MiB = 1,048,576 bytes. A number without a unit means bytes.
              </p>
              <fieldset
                className="dimension-fields"
                disabled={state === "processing"}
              >
                <legend>Width</legend>
                <label>
                  Exact
                  <input
                    aria-label="Exact width"
                    inputMode="numeric"
                    value={target.widthExact}
                    onChange={(event) =>
                      editTarget((current) => ({
                        ...current,
                        widthExact: event.target.value,
                      }))
                    }
                    placeholder="Any"
                  />
                </label>
                <label>
                  Minimum
                  <input
                    aria-label="Minimum width"
                    inputMode="numeric"
                    value={target.widthMin}
                    onChange={(event) =>
                      editTarget((current) => ({
                        ...current,
                        widthMin: event.target.value,
                      }))
                    }
                    placeholder="None"
                  />
                </label>
                <label>
                  Maximum
                  <input
                    aria-label="Maximum width"
                    inputMode="numeric"
                    value={target.widthMax}
                    onChange={(event) =>
                      editTarget((current) => ({
                        ...current,
                        widthMax: event.target.value,
                      }))
                    }
                    placeholder="None"
                  />
                </label>
              </fieldset>
              <fieldset
                className="dimension-fields"
                disabled={state === "processing"}
              >
                <legend>Height</legend>
                <label>
                  Exact
                  <input
                    aria-label="Exact height"
                    inputMode="numeric"
                    value={target.heightExact}
                    onChange={(event) =>
                      editTarget((current) => ({
                        ...current,
                        heightExact: event.target.value,
                      }))
                    }
                    placeholder="Any"
                  />
                </label>
                <label>
                  Minimum
                  <input
                    aria-label="Minimum height"
                    inputMode="numeric"
                    value={target.heightMin}
                    onChange={(event) =>
                      editTarget((current) => ({
                        ...current,
                        heightMin: event.target.value,
                      }))
                    }
                    placeholder="None"
                  />
                </label>
                <label>
                  Maximum
                  <input
                    aria-label="Maximum height"
                    inputMode="numeric"
                    value={target.heightMax}
                    onChange={(event) =>
                      editTarget((current) => ({
                        ...current,
                        heightMax: event.target.value,
                      }))
                    }
                    placeholder="None"
                  />
                </label>
              </fieldset>
              {targetDirty ? (
                <button
                  className="secondary"
                  type="button"
                  onClick={() => void replan()}
                  disabled={state === "processing" || Boolean(targetError)}
                >
                  Review target changes
                </button>
              ) : null}
            </div>
          ) : null}
          {targetError ? (
            <p className="error-copy" role="alert">
              {targetError}
            </p>
          ) : null}
        </div>
      ) : desktopTarget ? (
        <>
          <p className="target-summary">
            {desktopTarget.profile
              ? profilePlainSummary(desktopTarget.profile, setup.discordCap)
              : "Custom media constraints"}
          </p>
          <ProfileProvenance profileId={desktopTarget.profile ?? null} />
        </>
      ) : null}
    </section>
  );
}
