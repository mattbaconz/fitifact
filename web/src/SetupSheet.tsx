import type { DoctorReport } from "./lib/desktop-engine";
import {
  DISCORD_CAPS,
  destinationChips,
  type DestinationFamily,
  type DiscordCap,
} from "./lib/destinations";
import type { SetupState } from "./lib/setup";
import {
  ChatIcon,
  GitHubIcon,
  JpegIcon,
  MailIcon,
  SlackIcon,
  VideoIcon,
  WhatsAppIcon,
  XIcon,
} from "./icons";

const CHIP_ICONS: Record<DestinationFamily, typeof ChatIcon> = {
  discord: ChatIcon,
  gmail: MailIcon,
  github: GitHubIcon,
  slack: SlackIcon,
  whatsapp: WhatsAppIcon,
  x: XIcon,
  jpeg: JpegIcon,
  "generic-video": VideoIcon,
} as const;

interface SetupSheetProps {
  setup: SetupState;
  doctor: DoctorReport | null;
  doctorCopy: string;
  confirmLabel: string;
  showDoctor?: boolean;
  includeVideo?: boolean;
  onChange: (next: SetupState) => void;
  onConfirm: () => void;
  onDismiss?: () => void;
  onRecheck?: () => void | Promise<void>;
}

export function SetupSheet({
  setup,
  doctor,
  doctorCopy,
  confirmLabel,
  showDoctor = true,
  includeVideo = true,
  onChange,
  onConfirm,
  onDismiss,
  onRecheck,
}: SetupSheetProps) {
  const unhealthy = Boolean(doctor && !doctor.healthy);
  const videoBlocked = unhealthy;
  const discordSelected = setup.families.includes("discord");
  const chips = destinationChips(setup.discordCap, { includeVideo });

  function toggleFamily(family: DestinationFamily) {
    if (videoBlocked && family === "generic-video") return;
    const has = setup.families.includes(family);
    const families = has
      ? setup.families.filter((item) => item !== family)
      : [...setup.families, family];
    onChange({ ...setup, families });
  }

  function setCap(discordCap: DiscordCap) {
    onChange({ ...setup, discordCap });
  }

  async function copyInstall() {
    try {
      await navigator.clipboard.writeText(doctorCopy);
    } catch {
      /* clipboard can be denied; the commands stay visible */
    }
  }

  return (
    <section className="setup-sheet card" aria-labelledby="setup-title">
      <div className="setup-head">
        <h2 id="setup-title">Setup</h2>
        {onDismiss ? (
          <button type="button" className="ghost" onClick={onDismiss}>Close</button>
        ) : null}
      </div>

      {showDoctor ? (
      <div className="setup-block">
        <h3>Doctor</h3>
        {unhealthy ? (
          <>
            <p className="notice install-copy">{doctorCopy}</p>
            <p className="empty-copy">Images stay available. Video waits until ffmpeg and ffprobe are on PATH.</p>
            <div className="setup-doctor-actions">
              <button type="button" className="secondary" onClick={() => void copyInstall()}>Copy install commands</button>
              {onRecheck ? (
                <button type="button" className="secondary" onClick={() => void onRecheck()}>Check again</button>
              ) : null}
            </div>
          </>
        ) : doctor ? (
          <p className="empty-copy">ffmpeg and ffprobe are on PATH.</p>
        ) : (
          <p className="empty-copy">Checking local ffmpeg and ffprobe.</p>
        )}
      </div>
      ) : null}

      <div className="setup-block">
        <h3>Destinations you use</h3>
        <fieldset className="setup-families">
          <legend className="visually-hidden">Destinations you use</legend>
          <div className="destination-chips">
          {chips.map((chip) => {
            const Icon = CHIP_ICONS[chip.family];
            const selected = setup.families.includes(chip.family);
            const videoLocked = chip.videoOnly && videoBlocked;
            return (
              <button
                key={chip.family}
                type="button"
                className={`destination-chip${selected ? " is-selected" : ""}`}
                disabled={videoLocked}
                aria-pressed={selected}
                onClick={() => toggleFamily(chip.family)}
              >
                <span className="destination-chip-label"><Icon />{chip.label}</span>
                <span className="destination-chip-sub">{videoLocked ? "Needs ffmpeg on PATH" : chip.subtitle}</span>
              </button>
            );
          })}
          </div>
        </fieldset>
      </div>

      {discordSelected ? (
      <div className="setup-block">
        <h3>Discord upload cap</h3>
        <p className="empty-copy">Fitifact cannot see Nitro. Pick the ceiling your account actually has.</p>
        <fieldset className="setup-caps">
          <legend className="visually-hidden">Discord upload cap</legend>
          {(Object.keys(DISCORD_CAPS) as DiscordCap[]).map((cap) => (
            <label key={cap} className="check-label">
              <input
                type="radio"
                name="discord-cap"
                checked={setup.discordCap === cap}
                onChange={() => setCap(cap)}
              />
              {DISCORD_CAPS[cap].label} · {DISCORD_CAPS[cap].short}
            </label>
          ))}
        </fieldset>
      </div>
      ) : null}

      <p className="empty-copy setup-output-note">Adapted files are unique .fitifact. siblings. The original is never overwritten.</p>

      <button type="button" onClick={onConfirm}>{confirmLabel}</button>
    </section>
  );
}
