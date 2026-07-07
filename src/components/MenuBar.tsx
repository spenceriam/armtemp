import { useEffect, useRef, useState } from "react";

interface MenuBarProps {
  alwaysOnTop: boolean;
  onOpenSettings: () => void;
  onOpenOverheat: () => void;
  onOpenAbout: () => void;
  onOpenFeedback: () => void;
  onToggleMini: () => void;
  onToggleAlwaysOnTop: () => void;
  onRefresh: () => void;
  onExit: () => void;
}

type MenuKey = "file" | "options" | "tools" | "help";

// Win32-style menu bar (File / Options / Tools / Help) rendered as themed HTML
// dropdowns — a native HMENU doesn't follow app dark/light mode on Windows, so
// this reads as native chrome while staying theme-correct.
export function MenuBar({
  alwaysOnTop,
  onOpenSettings,
  onOpenOverheat,
  onOpenAbout,
  onOpenFeedback,
  onToggleMini,
  onToggleAlwaysOnTop,
  onRefresh,
  onExit,
}: MenuBarProps) {
  const [openMenu, setOpenMenu] = useState<MenuKey | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!openMenu) return;
    const onDocDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpenMenu(null);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpenMenu(null);
    };
    document.addEventListener("mousedown", onDocDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDocDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [openMenu]);

  const run = (fn: () => void) => {
    setOpenMenu(null);
    fn();
  };

  const topButton = (key: MenuKey, label: string) => (
    <button
      className={`menu-top ${openMenu === key ? "open" : ""}`}
      aria-haspopup="menu"
      aria-expanded={openMenu === key}
      onClick={() => setOpenMenu((m) => (m === key ? null : key))}
      onMouseEnter={() => setOpenMenu((m) => (m ? key : m))}
    >
      {label}
    </button>
  );

  return (
    <div className="menubar" ref={rootRef}>
      <div className="menu-wrap">
        {topButton("file", "File")}
        {openMenu === "file" && (
          <div className="menu-drop" role="menu">
            <button className="menu-entry" role="menuitem" onClick={() => run(onExit)}>
              Exit
            </button>
          </div>
        )}
      </div>
      <div className="menu-wrap">
        {topButton("options", "Options")}
        {openMenu === "options" && (
          <div className="menu-drop" role="menu">
            <button className="menu-entry" role="menuitem" onClick={() => run(onOpenSettings)}>
              Settings
            </button>
            <button className="menu-entry" role="menuitem" onClick={() => run(onOpenOverheat)}>
              Overheat protection
            </button>
            <div className="menu-sep" />
            <button className="menu-entry" role="menuitem" onClick={() => run(onToggleMini)}>
              Toggle Mini Mode
            </button>
            <button
              className={`menu-entry ${alwaysOnTop ? "menu-check" : ""}`}
              role="menuitemcheckbox"
              aria-checked={alwaysOnTop}
              onClick={() => run(onToggleAlwaysOnTop)}
            >
              Always on top
            </button>
          </div>
        )}
      </div>
      <div className="menu-wrap">
        {topButton("tools", "Tools")}
        {openMenu === "tools" && (
          <div className="menu-drop" role="menu">
            <button className="menu-entry" role="menuitem" onClick={() => run(onRefresh)}>
              Refresh sensors
            </button>
          </div>
        )}
      </div>
      <div className="menu-wrap">
        {topButton("help", "Help")}
        {openMenu === "help" && (
          <div className="menu-drop" role="menu">
            <button className="menu-entry" role="menuitem" onClick={() => run(onOpenFeedback)}>
              Feedback
            </button>
            <div className="menu-sep" />
            <button className="menu-entry" role="menuitem" onClick={() => run(onOpenAbout)}>
              About ARMtemp
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
