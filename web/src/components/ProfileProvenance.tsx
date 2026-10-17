import {
  ageLabel,
  daysSince,
  describeRule,
  isStale,
  registryProfile,
  SOURCE_LABEL,
  STALE_AFTER_DAYS,
  TRUST_LABEL,
} from "../lib/registry";

export function ProfileProvenance({ profileId }: { profileId: string | null }) {
  const profile = registryProfile(profileId);
  if (!profile) return null;
  const stale = isStale(profile);
  return (
    <details className="provenance">
      <summary>
        Rules checked {ageLabel(daysSince(profile.last_verified))}
        {stale ? <span className="stale-badge">Stale</span> : null}
      </summary>
      <p className="helper">
        {TRUST_LABEL[profile.trust] ?? profile.trust}
        {stale
          ? ` Not re-checked in over ${STALE_AFTER_DAYS} days, so verify these limits before relying on them.`
          : ""}
      </p>
      <ul>
        {profile.rules.map((rule) => (
          <li key={rule.id}>
            <span>{describeRule(rule)}</span>
            <span className="helper">
              {SOURCE_LABEL[rule.source_type ?? ""] ?? "Unrecorded source"}
              {rule.observed_at ? `, checked ${ageLabel(daysSince(rule.observed_at))}` : ""}
              {rule.source_url ? (
                <>
                  {" · "}
                  <a href={rule.source_url} target="_blank" rel="noreferrer">
                    Source
                  </a>
                </>
              ) : null}
            </span>
          </li>
        ))}
      </ul>
    </details>
  );
}
