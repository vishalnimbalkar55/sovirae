// Inline stroke icons (24-unit grid) so no icon font or network is needed.
import type { SVGProps } from "react";

type P = SVGProps<SVGSVGElement>;
const base = (p: P) => ({
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.8,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true,
  ...p,
});

export const Play = (p: P) => (
  <svg {...base(p)}><path d="M8 5.5v13l10.5-6.5z" fill="currentColor" stroke="none" /></svg>
);
export const Pause = (p: P) => (
  <svg {...base(p)}><rect x="6.5" y="5.5" width="3.8" height="13" rx="1" fill="currentColor" stroke="none" /><rect x="13.7" y="5.5" width="3.8" height="13" rx="1" fill="currentColor" stroke="none" /></svg>
);
export const Back10 = (p: P) => (
  <svg {...base(p)}><path d="M4 12a8 8 0 1 0 2.4-5.7" /><path d="M4 4.5v4h4" /><text x="12" y="15.2" fontSize="7" textAnchor="middle" fill="currentColor" stroke="none" fontWeight="700" fontFamily="system-ui">10</text></svg>
);
export const Forward10 = (p: P) => (
  <svg {...base(p)}><path d="M20 12a8 8 0 1 1-2.4-5.7" /><path d="M20 4.5v4h-4" /><text x="12" y="15.2" fontSize="7" textAnchor="middle" fill="currentColor" stroke="none" fontWeight="700" fontFamily="system-ui">10</text></svg>
);
export const Close = (p: P) => (
  <svg {...base(p)}><path d="M6.5 6.5l11 11M17.5 6.5l-11 11" /></svg>
);
export const Expand = (p: P) => (
  <svg {...base(p)}><path d="M14 5h5v5M10 19H5v-5M19 5l-6 6M5 19l6-6" /></svg>
);
export const Collapse = (p: P) => (
  <svg {...base(p)}><path d="M19 10h-5V5M5 14h5v5M14 10l6-6M10 14l-6 6" /></svg>
);
export const Volume = (p: P) => (
  <svg {...base(p)}><path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z" /><path d="M15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11" /></svg>
);
export const Mute = (p: P) => (
  <svg {...base(p)}><path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z" /><path d="M16 9.5l5 5M21 9.5l-5 5" /></svg>
);
export const ReadIcon = (p: P) => (
  <svg {...base(p)}><path d="M4 5.5h6.5a2 2 0 0 1 2 2V19a1.8 1.8 0 0 0-1.8-1.8H4zM20 5.5h-6.5a2 2 0 0 0-2 2V19a1.8 1.8 0 0 1 1.8-1.8H20z" /></svg>
);
export const VoiceIcon = (p: P) => (
  <svg {...base(p)}><path d="M5 10v4M8.5 7v10M12 4.5v15M15.5 8v8M19 10.5v3" /></svg>
);
export const KeyIcon = (p: P) => (
  <svg {...base(p)}><rect x="3" y="6.5" width="18" height="11" rx="2.5" /><path d="M7 10h.01M10.5 10h.01M14 10h.01M17 10h.01M8 14h8" /></svg>
);
export const PuzzleIcon = (p: P) => (
  <svg {...base(p)}><path d="M9 4.5h3.5v2a1.8 1.8 0 1 0 3.5 0v-2H19.5V9h-2a1.8 1.8 0 1 0 0 3.5h2v7H16v-2a1.8 1.8 0 1 0-3.5 0v2H4.5V13h2a1.8 1.8 0 1 0 0-3.5h-2V4.5z" /></svg>
);
export const GearIcon = (p: P) => (
  <svg {...base(p)}><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></svg>
);
export const Clipboard = (p: P) => (
  <svg {...base(p)}><rect x="6" y="4.5" width="12" height="16" rx="2" /><path d="M9.5 4.5V3.8a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1v.7" /></svg>
);
export const Doc = (p: P) => (
  <svg {...base(p)}><path d="M7 3.5h7l4 4V20a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V4.5a1 1 0 0 1 1-1z" /><path d="M14 3.5v4h4M9 12h6M9 15.5h6" /></svg>
);
export const Check = (p: P) => (
  <svg {...base(p)}><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
);
export const Alert = (p: P) => (
  <svg {...base(p)}><path d="M12 4l9 16H3z" /><path d="M12 10v4.5M12 17.5h.01" /></svg>
);

/** Sovirae mark (provisional): a lowercase "s" whose top terminal is a
 *  text caret, i.e. a reading mark becoming sound (branding spec §5.1). */
export const Mark = (p: P) => (
  <svg viewBox="0 0 24 24" fill="none" aria-hidden {...p}>
    <path
      d="M15.5 3.5v3.2c0 1-.9 1.6-3 1.6-2.4 0-4.8.9-4.8 3.3 0 2.2 2 2.8 4.3 3.3 2.4.5 4.5 1.3 4.5 3.5 0 2.3-2.2 3.3-4.7 3.3-2 0-3.6-.6-4.6-1.7"
      stroke="currentColor"
      strokeWidth="2.3"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  </svg>
);
export const Chevron = (p: P) => (
  <svg {...base(p)}><path d="M9 6l6 6-6 6" /></svg>
);
export const Download = (p: P) => (
  <svg {...base(p)}><path d="M12 4v11M7.5 10.5L12 15l4.5-4.5M5 19.5h14" /></svg>
);
export const StudioIcon = (p: P) => (
  <svg {...base(p)}><path d="M4 5.5h16M4 10h10M4 14.5h16M4 19h7" /><circle cx="18" cy="11" r="2.2" /></svg>
);
export const Copy = (p: P) => (
  <svg {...base(p)}><rect x="9" y="9" width="11" height="11" rx="2" /><path d="M5 15V6a2 2 0 0 1 2-2h9" /></svg>
);
export const Trash = (p: P) => (
  <svg {...base(p)}><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></svg>
);
export const Stop = (p: P) => (
  <svg {...base(p)}><rect x="6" y="6" width="12" height="12" rx="2" fill="currentColor" stroke="none" /></svg>
);
export const Plus = (p: P) => (
  <svg {...base(p)}><path d="M12 5v14M5 12h14" /></svg>
);
