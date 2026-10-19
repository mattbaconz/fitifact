import { useEffect, useState } from "react";
import { FileSummary } from "./components/FileSummary";
import { ProcessRail } from "./components/ProcessRail";
import { ProductExamples, ProductIntro } from "./components/ProductIntro";
import { RequirementsPanel } from "./components/RequirementsPanel";
import { ResultPanel, StatusIcon } from "./components/ResultPanel";
import { SettingsDrawer } from "./components/SettingsDrawer";
import { SetupDialog } from "./components/SetupDialog";
import { BrandMark, DropIcon, MenuIcon } from "./icons";
import {
  desktopGuideHref,
  needsDesktopHandoff,
  publicDocsHref,
} from "./lib/docs-url";
import { summarizeTarget } from "./lib/target-summary";
import { SetupSheet } from "./SetupSheet";
import { RejectBlock, useFitifactSession } from "./useFitifactSession";

export function App() {
  const session = useFitifactSession();
  const {
    resultReady,
    showWork,
    focusPasteOverride,
    sidebarOpen,
    setSidebarOpen,
    notice,
    setNotice,
    dragging,
    state,
    tone,
    inspectingName,
    onDropKey,
    setDragging,
    onDrop,
    desktop,
    chooseDesktopFile,
    chooseLabel,
    fileInputRef,
    accept,
    inspectFile,
    useSampleImage,
    statusTitle,
    progress,
    mediaPath,
    cancel,
    lastRejected,
    error,
    target,
    setupOpen,
    setSetupOpen,
    setup,
    doctor,
    setSetup,
    commitSetup,
    refreshDoctor,
  } = session;
  const [mobile, setMobile] = useState(
    () => window.matchMedia("(max-width: 760px)").matches,
  );
  const [expanded, setExpanded] = useState(false);
  useEffect(() => {
    const query = window.matchMedia("(max-width: 760px)");
    const change = () => setMobile(query.matches);
    change();
    query.addEventListener("change", change);
    return () => query.removeEventListener("change", change);
  }, []);
  useEffect(() => {
    setExpanded(false);
  }, [resultReady]);
  useEffect(() => {
    if (showWork) {
      window.scrollTo({ top: 0 });
      document.getElementById("top")?.focus({ preventScroll: true });
    }
  }, [showWork]);
  useEffect(() => {
    if (state !== "adapted") return;
    const frame = requestAnimationFrame(() => {
      const heading = document.getElementById("status-title");
      heading?.focus({ preventScroll: true });
      heading?.scrollIntoView({ block: "start", behavior: "instant" });
    });
    return () => cancelAnimationFrame(frame);
  }, [state]);
  function changeRequirements() {
    setExpanded(true);
    focusPasteOverride();
  }
  return (
    <div className={`app-shell ${showWork ? "has-file" : "is-idle"}`}>
      <a className="skip-link" href="#top">
        Skip to workspace
      </a>
      <header className="site-header" inert={sidebarOpen}>
        <a
          className="wordmark"
          href={import.meta.env.BASE_URL}
          aria-label="Fitifact home"
        >
          <BrandMark className="brand-mark" />
          <span>Fitifact</span>
        </a>
        <nav className="desktop-nav" aria-label="Main navigation">
          <a
            href={
              showWork
                ? `${publicDocsHref()}plans-and-reports/`
                : "#how-it-works"
            }
          >
            How it works
          </a>
          <a href={publicDocsHref()}>Docs</a>
          <a href={desktopGuideHref()}>Downloads</a>
        </nav>
        <button
          type="button"
          className="ft-btn ft-btn-secondary menu-button"
          aria-label="Settings and navigation"
          aria-expanded={sidebarOpen}
          aria-controls="app-sidebar"
          onClick={() => setSidebarOpen((open) => !open)}
        >
          <MenuIcon /> <span>Settings</span>
        </button>
      </header>
      {notice ? (
        <div className="global-notice" role="status">
          {notice}
          <button
            type="button"
            className="ft-btn ft-btn-text"
            onClick={() => setNotice("")}
          >
            Dismiss
          </button>
        </div>
      ) : null}
      <main id="top" tabIndex={-1} inert={sidebarOpen}>
        {!showWork ? (
          <>
            <section className="landing-workspace">
              <ProductIntro />
              <div className="drop-canvas">
                {state === "processing" ? (
                  <div
                    className="ft-card inspect-card"
                    role="status"
                    aria-labelledby="inspect-title"
                    // A file dropped while inspecting is ignored, never opened by the browser.
                    onDragOver={(event) => event.preventDefault()}
                    onDrop={(event) => event.preventDefault()}
                  >
                    <span className="inspect-pulse" aria-hidden="true" />
                    <p className="ft-eyebrow">On this device</p>
                    <h2 id="inspect-title" className="status-title">
                      {inspectingName
                        ? `Inspecting ${inspectingName}`
                        : "Inspecting your file"}
                    </h2>
                    <p>{progress?.stage ?? "Reading the file locally"}</p>
                    <progress aria-label="Processing file" />
                    {!mediaPath ? (
                      <button
                        type="button"
                        className="ft-btn ft-btn-secondary"
                        onClick={() => void cancel()}
                      >
                        Cancel processing
                      </button>
                    ) : null}
                  </div>
                ) : (
                <div
                  className={`drop-zone drop-zone-hero ${dragging ? "is-dragging" : ""}`}
                  aria-label="Drop a file"
                  tabIndex={0}
                  onKeyDown={onDropKey}
                  onDragOver={(event) => {
                    event.preventDefault();
                    setDragging(true);
                  }}
                  onDragLeave={() => setDragging(false)}
                  onDrop={onDrop}
                >
                  <DropIcon className="drop-glyph" />
                  <p>Drop a file</p>
                  <p id="image-help">
                    {desktop
                      ? "Photos, video, and audio · JPEG PNG WebP HEIC TIFF BMP GIF · MP4 MOV WebM MKV · MP3 WAV FLAC M4A"
                      : "Photos and video · JPEG PNG WebP HEIC TIFF BMP GIF · MP4 MOV WebM MKV · photos up to 32 MiB or 24 megapixels"}
                  </p>
                  {desktop ? (
                    <button
                      className="ft-btn button-label"
                      type="button"
                      onClick={() => void chooseDesktopFile()}
                    >
                      {chooseLabel}
                    </button>
                  ) : (
                    <label className="ft-btn button-label" htmlFor="image-file">
                      {chooseLabel}
                    </label>
                  )}
                  <input
                    hidden={desktop}
                    id="image-file"
                    ref={fileInputRef}
                    className="visually-hidden"
                    type="file"
                    accept={accept}
                    aria-describedby="image-help"
                    onChange={(event) => {
                      const file = event.currentTarget.files?.item(0);
                      if (file) void inspectFile(file);
                      event.currentTarget.value = "";
                    }}
                  />
                </div>
                )}

                <button
                  className="ft-btn ft-btn-text drop-sample"
                  type="button"
                  onClick={() => void useSampleImage()}
                  disabled={state === "processing"}
                >
                  Try a sample image <span aria-hidden="true">↗</span>
                </button>
                <p className="browser-capability">
                  {desktop ? (
                    "Photos, video, and audio. All on this computer."
                  ) : (
                    <>
                      Photos and video in this browser.{" "}
                      <a href={desktopGuideHref()}>
                        Re-encoding and audio on desktop.
                      </a>
                    </>
                  )}
                </p>
                <div className="idle-status" role="status" aria-live="polite">
                  {state !== "idle" && state !== "processing" ? (
                    <div className={`ft-card idle-result tone-${tone}`}>
                      <div className="status-head">
                        <StatusIcon tone={tone} />
                        <h2 className="status-title">{statusTitle}</h2>
                      </div>
                      {lastRejected ? (
                        <RejectBlock
                          name={lastRejected.name}
                          message={lastRejected.message}
                          offerDesktop={
                            !desktop && needsDesktopHandoff(lastRejected.message)
                          }
                        />
                      ) : error ? (
                        <p className="ft-notice ft-tone-danger error-copy">
                          {error.message}
                        </p>
                      ) : null}
                    </div>
                  ) : null}
                </div>
              </div>
            </section>
            <div className="intro-process">
              <ProcessRail
                state={state}
                stage={progress?.stage}
                hasFile={false}
              />
              <p>
                Understand the file. Review the plan. Change only what’s needed.
                Verify the result.
              </p>
            </div>
            <ProductExamples
              onSample={(scenario) => void useSampleImage(scenario)}
              disabled={state === "processing"}
            />
          </>
        ) : (
          <div className="work-surface">
            <div className="workspace-heading">
              <h1>Adapt a file</h1>
              <p className="local-note">On this device · original preserved</p>
            </div>
            <FileSummary session={session} />

            <ProcessRail
              state={state}
              stage={progress?.stage}
              hasFile={showWork}
            />
            <div className="workspace-columns">
              <div className="requirements-column">
                <details
                  className="requirements-disclosure"
                  open={!mobile || !resultReady || expanded}
                >
                  <summary onClick={(event) => {
                    event.preventDefault();
                    if (mobile && resultReady) setExpanded((open) => !open);
                  }}>
                    Current requirements
                    <span>
                      {target ? summarizeTarget(target) : "Destination rules"}
                    </span>
                  </summary>
                  <RequirementsPanel
                    session={session}
                    changeRequirements={changeRequirements}
                  />
                </details>
              </div>
              <div className="result-column">
                <ResultPanel
                  session={session}
                  changeRequirements={changeRequirements}
                />
              </div>
            </div>
          </div>
        )}
      </main>
      <footer className="site-footer" inert={sidebarOpen}>
        <span>
          Fitifact <span className="muted">/</span> Minimum change. Verified
          fit.
        </span>
        <nav aria-label="Footer">
          <a href={publicDocsHref()}>Docs</a>
          <a href="https://github.com/mattbaconz/fitifact">GitHub</a>
          <a href={`${publicDocsHref()}privacy-and-limitations/`}>Privacy</a>
        </nav>
        <p>Local · nothing is uploaded</p>
      </footer>
      {setupOpen ? (
        <SetupDialog onClose={() => setSetupOpen(false)}>
          <SetupSheet
            setup={setup}
            doctor={doctor}
            confirmLabel="Save preferences"
            onChange={setSetup}
            onConfirm={() => {
              commitSetup(setup, true);
              setSetupOpen(false);
            }}
            onDismiss={() => setSetupOpen(false)}
            onRecheck={refreshDoctor}
          />
        </SetupDialog>
      ) : null}
      <SettingsDrawer session={session} />
    </div>
  );
}
