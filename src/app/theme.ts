// Design tokens ported from the Claude design's vars()/hexA()/shade()/tcolor().
// These are the exact colors and thresholds the mockup used.

export type Theme = "dark" | "light";

interface TokenSet {
  [token: string]: string;
}

// Win32 classic-dialog palette (Core Temp look) — flat surfaces, etched group
// boxes, sunken value fields. Dark mode is our own addition (real Core Temp
// has none); values chosen to read as a native dark dialog, not a website.
const DARK: TokenSet = {
  "--mica": "#202020", // kept for API compat; window is opaque now
  "--surface": "#202020", // dialog background
  "--surface-2": "#262626",
  "--card": "#2b2b2b", // menus / dropdowns
  "--card-2": "#262626",
  "--sunken-bg": "#191919", // sunken value-field background
  "--groove-hi": "#3f3f3f", // group-box etched highlight
  "--groove-lo": "#0d0d0d", // group-box etched shadow
  "--text": "#ffffff",
  "--text-2": "rgba(255,255,255,0.82)",
  "--text-3": "rgba(255,255,255,0.55)",
  "--border": "#3a3a3a",
  "--border-2": "#454545",
  "--hover": "rgba(255,255,255,0.08)",
  "--track": "rgba(255,255,255,0.22)",
  "--shadow": "none",
};
const LIGHT: TokenSet = {
  "--mica": "#f0f0f0",
  "--surface": "#f0f0f0", // classic Windows dialog gray
  "--surface-2": "#e8e8e8",
  "--card": "#ffffff",
  "--card-2": "#f5f5f5",
  "--sunken-bg": "#ffffff",
  "--groove-hi": "#ffffff",
  "--groove-lo": "#a0a0a0",
  "--text": "#1a1a1a",
  "--text-2": "rgba(0,0,0,0.75)",
  "--text-3": "rgba(0,0,0,0.5)",
  "--border": "#d4d4d4",
  "--border-2": "#adadad",
  "--hover": "rgba(0,0,0,0.06)",
  "--track": "rgba(0,0,0,0.3)",
  "--shadow": "none",
};

export const ACCENT = "#0078D4"; // Windows accent blue (design default).

/// Build the CSS variable map for a given theme + accent. Returns a React style object.
export function themeVars(theme: Theme, accent: string = ACCENT): React.CSSProperties {
  const base = theme === "dark" ? DARK : LIGHT;
  const vars: Record<string, string> = { ...base };
  vars["--accent"] = accent;
  vars["--accent-2"] = shade(accent, -22);
  vars["--accent-soft"] = hexA(accent, theme === "dark" ? 0.22 : 0.12);
  // Cast: React.CSSProperties typed, but we use CSS custom properties.
  return vars as unknown as React.CSSProperties;
}

/// "#rrggbb" + alpha -> rgba() string. Ported from the design's hexA().
export function hexA(hex: string, a: number): string {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${a})`;
}

/// Lighten/darken a "#rrggbb" by percent. Ported from the design's shade().
export function shade(hex: string, p: number): string {
  let n = parseInt(hex.slice(1), 16);
  const r = (n >> 16) & 255,
    g = (n >> 8) & 255,
    b = n & 255;
  const f = (c: number) => Math.max(0, Math.min(255, Math.round(c + (c * p) / 100)));
  return (
    "#" +
    [f(r), f(g), f(b)]
      .map((x) => x.toString(16).padStart(2, "0"))
      .join("")
  );
}

/// Temperature -> color, ported from the design's tcolor(). t in °C, tj is TjMax.
/// Returns green/yellow/orange/red based on proximity to TjMax.
export function tempColor(t: number, tj: number): string {
  const r = (t - 35) / (tj - 35);
  if (r < 0.45) return "#2fa45a";
  if (r < 0.64) return "#d8a51a";
  if (r < 0.82) return "#e07a2b";
  return "#e0473a";
}

/// Format a Celsius temperature for display, honoring the user's unit choice.
export function formatTemp(c: number | null | undefined, unit: "C" | "F"): string {
  if (c === null || c === undefined) return "—";
  return unit === "C" ? `${Math.round(c)}°C` : `${Math.round((c * 9) / 5 + 32)}°F`;
}

/// Format a temperature as a bare number (for compact tray/toolbar display).
export function formatTempNum(c: number | null | undefined, unit: "C" | "F"): string {
  if (c === null || c === undefined) return "—";
  return unit === "C" ? `${Math.round(c)}` : `${Math.round((c * 9) / 5 + 32)}`;
}
