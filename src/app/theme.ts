// Design tokens ported from the Clod design's vars()/hexA()/shade()/tcolor().
// These are the exact colors and thresholds the mockup used.

export type Theme = "dark" | "light";

interface TokenSet {
  [token: string]: string;
}

// The CSS-variable palette from the design's vars(), verbatim.
const DARK: TokenSet = {
  "--mica": "rgba(43,43,43,0.74)",
  "--surface": "#272727",
  "--surface-2": "#2f2f2f",
  "--card": "#2d2d2d",
  "--card-2": "#333333",
  "--text": "#ffffff",
  "--text-2": "rgba(255,255,255,0.78)",
  "--text-3": "rgba(255,255,255,0.5)",
  "--border": "rgba(255,255,255,0.07)",
  "--border-2": "rgba(255,255,255,0.13)",
  "--hover": "rgba(255,255,255,0.06)",
  "--track": "rgba(255,255,255,0.1)",
  "--shadow": "0 20px 60px rgba(0,0,0,0.6)",
};
const LIGHT: TokenSet = {
  "--mica": "rgba(249,249,249,0.78)",
  "--surface": "#f3f3f3",
  "--surface-2": "#eaeaea",
  "--card": "#ffffff",
  "--card-2": "#f7f7f7",
  "--text": "#1b1b1b",
  "--text-2": "rgba(0,0,0,0.68)",
  "--text-3": "rgba(0,0,0,0.45)",
  "--border": "rgba(0,0,0,0.07)",
  "--border-2": "rgba(0,0,0,0.13)",
  "--hover": "rgba(0,0,0,0.045)",
  "--track": "rgba(0,0,0,0.1)",
  "--shadow": "0 20px 60px rgba(0,0,0,0.25)",
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
