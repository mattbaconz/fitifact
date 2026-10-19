import { ThemeControl } from "../components/ThemeControl";
import { ToolDiagnostics } from "./ToolDiagnostics";
import { desktopGuideHref, publicDocsHref } from "../lib/docs-url";
import { SetupSheet } from "../SetupSheet";
import type { FitifactSession } from "../useFitifactSession";
export function SettingsDrawer({ session }: { session: FitifactSession }) {
  const {
    sidebarOpen,
    setSidebarOpen,
    sidebarRef,
    settings,
    updateSettings,
    setup,
    doctor,
    desktop,
    setSetup,
    commitSetup,
    setNotice,
    refreshDoctor,
  } = session;
  return (
    <div
      className={`sidebar-overlay${sidebarOpen ? " is-open" : ""}`}
      onClick={() => setSidebarOpen(false)}
      aria-hidden={!sidebarOpen}
      inert={!sidebarOpen}
    >
      <aside
        id="app-sidebar"
        className="sidebar"
        ref={sidebarRef}
        role={sidebarOpen ? "dialog" : undefined}
        aria-modal={sidebarOpen ? true : undefined}
        aria-labelledby="sidebar-title"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="sidebar-head">
          <h2 id="sidebar-title">Settings</h2>
          <button className="ft-btn ft-btn-secondary" onClick={() => setSidebarOpen(false)}>
            Close
          </button>
        </div>
        <nav className="drawer-nav" aria-label="More navigation">
          <a href={`${publicDocsHref()}plans-and-reports/`}>How it works</a>
          <a href={publicDocsHref()}>Docs</a>
          <a href={desktopGuideHref()}>Downloads</a>
        </nav>
        <section>
          <h3>Appearance</h3>
          <ThemeControl />
        </section>
        <section>
          <h3>Files</h3>
          <label className="check-label">
            <input
              type="checkbox"
              checked={settings.fitifactSuffix}
              onChange={(event) =>
                updateSettings({
                  ...settings,
                  fitifactSuffix: event.target.checked,
                })
              }
            />
            Add .fitifact to adapted download names
          </label>
          <p className="helper">
            Unchanged files keep their original name. Crop and frame removal
            always need approval for the current file.
          </p>
        </section>
        <section>
          <h3>Destination preferences</h3>
          <SetupSheet
            setup={setup}
            doctor={doctor}
            showDoctor={false}
            includeVideo={desktop}
            confirmLabel="Save destinations"
            onChange={setSetup}
            onConfirm={() => {
              commitSetup(setup, true);
              setNotice("Destination preferences saved.");
              setSidebarOpen(false);
            }}
            onRecheck={refreshDoctor}
          />
        </section>
        {desktop ? (
          <section>
            <h3>Local video tools</h3>
            <ToolDiagnostics doctor={doctor} onRecheck={refreshDoctor} />
          </section>
        ) : null}
        <section>
          <h3>About Fitifact</h3>
          <p>Your file stays on this device. Nothing is uploaded.</p>
          <p className="helper">
            Version {__FITIFACT_VERSION__} · Images are processed locally.{" "}
            {desktop
              ? "Video and audio use FFmpeg shipped with this app."
              : "Video is checked and repackaged in this browser. Re-encoding and audio use the desktop app."}
          </p>
          <a href={`${import.meta.env.BASE_URL}THIRD_PARTY_NOTICES.md`}>
            Third-party notices
          </a>
        </section>
      </aside>
    </div>
  );
}
