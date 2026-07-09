// Strips web-browser affordances the WebView2 host exposes by default, so the
// app reads as a native Windows program rather than a page in a browser
// shell. Purely behavioral — no styling changes. Call once from main.tsx.

const EDITABLE_SELECTOR = 'input, textarea, [contenteditable="true"]';

function isEditableTarget(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(EDITABLE_SELECTOR) !== null;
}

// Browser hotkeys that have no place in a native utility: reload, find,
// print, view-source, dev tools, caret browsing, zoom.
function isBlockedHotkey(e: KeyboardEvent): boolean {
  const mod = e.ctrlKey || e.metaKey;
  if (["F5", "F3", "F7", "F12"].includes(e.key)) return true;
  if (mod && ["r", "f", "p", "u", "j", "g", "+", "-", "=", "0"].includes(e.key.toLowerCase())) {
    return true;
  }
  return false;
}

export function installNativeShell(): void {
  // No browser right-click menu — except where it's genuinely useful (text
  // inputs still get Cut/Copy/Paste).
  document.addEventListener(
    "contextmenu",
    (e) => {
      if (!isEditableTarget(e.target)) e.preventDefault();
    },
    { capture: true }
  );

  document.addEventListener(
    "keydown",
    (e) => {
      if (isBlockedHotkey(e)) e.preventDefault();
    },
    { capture: true }
  );

  // Ctrl+wheel zoom (and WebView2's trackpad-pinch-as-ctrl+wheel).
  window.addEventListener(
    "wheel",
    (e) => {
      if (e.ctrlKey) e.preventDefault();
    },
    { passive: false }
  );
}
