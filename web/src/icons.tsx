import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

function icon(props: IconProps) {
  return {
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.75,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true as const,
    ...props,
  };
}

export function BrandMark({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 384 398" width="21" height="22" fill="currentColor" aria-hidden="true">
      <path d="M50 3H143V70H72V113H143V180H72V395H2V51A48 48 0 0 1 50 3Z" />
      <path d="M162 70H232V113H280L290 145L280 180H232V395H162V70Z" />
      <path d="M311 329H381V395H311V329Z" />
    </svg>
  );
}

export function MenuIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M4 7h16M4 12h16M4 17h16" />
    </svg>
  );
}

export function CloseIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M6 6l12 12M18 6L6 18" />
    </svg>
  );
}

export function DropIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M12 3v12" />
      <path d="M7 10l5 5 5-5" />
      <path d="M5 19h14" />
    </svg>
  );
}

export function DownloadIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M12 4v10" />
      <path d="M8 10l4 4 4-4" />
      <path d="M5 18h14" />
    </svg>
  );
}

export function CheckPassIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M5 12.5l4.2 4.2L19 7.5" />
    </svg>
  );
}

export function CheckFailIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M7 7l10 10M17 7L7 17" />
    </svg>
  );
}

export function CheckUnknownIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M9 9a3 3 0 1 1 4.2 2.75c-.8.4-1.2.95-1.2 1.75V14" />
      <path d="M12 17.5h.01" />
    </svg>
  );
}

export function JpegIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <rect x="4" y="5" width="16" height="14" rx="2" />
      <path d="M8 15l2.5-3 2 2.2L15 11l3 4" />
    </svg>
  );
}

export function PngIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <rect x="4" y="5" width="16" height="14" rx="2" />
      <path d="M8 15V9h3a2 2 0 0 1 0 4H8" />
    </svg>
  );
}

export function WebpIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <rect x="4" y="5" width="16" height="14" rx="2" />
      <path d="M8 15V9l2 4 2-4v6" />
    </svg>
  );
}

export function ChatIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M5 6.5A2.5 2.5 0 0 1 7.5 4h9A2.5 2.5 0 0 1 19 6.5v7A2.5 2.5 0 0 1 16.5 16H11l-4 4v-4H7.5A2.5 2.5 0 0 1 5 13.5z" />
    </svg>
  );
}

export function MailIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <rect x="3.5" y="6" width="17" height="12" rx="2" />
      <path d="M4 7l8 6 8-6" />
    </svg>
  );
}

export function VideoIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <rect x="3.5" y="6" width="11" height="12" rx="1.5" />
      <path d="M14.5 10l6-3v10l-6-3z" />
    </svg>
  );
}

export function GitHubIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M9 19c-4.3 1.4-4.3-2.1-6-2.5" />
      <path d="M15 21v-3.5c0-1 .1-1.4-.5-2 2.8-.3 5.5-1.4 5.5-6a4.6 4.6 0 0 0-1.3-3.2 4.2 4.2 0 0 0-.1-3.2s-1.1-.3-3.5 1.3a12 12 0 0 0-6.2 0C6.5 2.8 5.4 3.1 5.4 3.1a4.2 4.2 0 0 0-.1 3.2A4.6 4.6 0 0 0 4 9.3c0 4.6 2.7 5.7 5.5 6-.6.5-.6 1.2-.5 2V21" />
    </svg>
  );
}

export function SlackIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M8 8h3v3H8zM13 8h3v3h-3zM8 13h3v3H8zM13 13h3v3h-3z" />
      <rect x="4.5" y="4.5" width="15" height="15" rx="2" />
    </svg>
  );
}

export function WhatsAppIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M6.5 17.5l-1.2 3.2 3.3-1.1A8.5 8.5 0 1 0 6.5 17.5z" />
      <path d="M9.2 10.2c.2-.5.3-.5.6-.5h.5c.2 0 .4.1.5.4l.7 1.6c.1.2 0 .4-.1.6l-.4.5c-.1.1-.1.3 0 .4.4.6 1 1.2 1.6 1.6.1.1.3.1.4 0l.5-.4c.2-.2.4-.2.6-.1l1.6.7c.3.1.4.3.4.5v.5c0 .3 0 .4-.5.6A3.8 3.8 0 0 1 13 16.2a7 7 0 0 1-3.8-3.8 3.8 3.8 0 0 1 0-2.2z" />
    </svg>
  );
}

export function XIcon(props: IconProps) {
  return (
    <svg {...icon(props)}>
      <path d="M5 5l14 14M19 5L5 19" />
    </svg>
  );
}
