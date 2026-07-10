# AGENTS.md
## Setup commands
- Install dependencies: `npm install`
- Start development server: `npm run tauri dev` (starts Vite on `127.0.0.1:1420` + the native Rust backend with hot-reload; check if already running)
- Build for production: `npm run tauri build` (outputs native ARM64 `.exe` + MSI/NSIS installers to `src-tauri/target/release/`)
- Run frontend-only type checking + build: `npm run build`
- Run Rust check (no codegen): `cargo check --manifest-path src-tauri/Cargo.toml`
- Run linter: `cargo clippy --manifest-path src-tauri/Cargo.toml`
## Project overview
ARMtemp is a native temperature monitor for Snapdragon X / X2 processors (Qualcomm Oryon) on Windows on ARM (ARM64). Built with Tauri 2 (Rust backend) + React 18 (TypeScript frontend), it reads real on-die thermal sensors via ACPI thermal zones and displays per-core temperatures, loads, and power. The app recreates the Core Temp experience for the Snapdragon X family, with a live system-tray icon, mini-mode, and classic Win32-style dialogs: a 4-tab Settings dialog (General / Display / Notification Area / Windows Taskbar, native checkboxes, OK/Cancel/Apply), a separate Overheat protection dialog (Options menu), and an About dialog (Help menu). All telemetry is strictly real — no simulated or fallback values anywhere.
## Development workflow discipline
- **CRITICAL**: NEVER commit or push changes without explicit user approval
- **ALWAYS** ask for user confirmation before any git operations
- **DEBUGGING**: Use console logs and testing to verify fixes before committing
- **WORKFLOW**: Make changes → Test → Get user approval → Then (and only then) commit → Push
- **Branch management**: Only commit to the correct issue branch
- **Code quality**: Ensure all changes work and are properly tested before seeking approval
## Version Bumping Protocol
**CRITICAL**: AI agents MUST follow this semantic versioning workflow when creating PRs.
### Semantic Versioning Format
Version numbers follow the format: **MAJOR.MINOR.PATCH** (e.g., 1.2.3)
- **MAJOR** (X.0.0): Breaking changes, API changes, architectural overhauls
  - Example: 1.5.2 → 2.0.0
- **MINOR** (0.X.0): New features, new components, significant enhancements (backwards compatible)
  - Example: 1.5.2 → 1.6.0
- **PATCH** (0.0.X): Bug fixes, typos, minor tweaks, performance improvements (backwards compatible)
  - Example: 1.5.2 → 1.5.3
### AI Agent Workflow for Version Bumping
**Step 1: Analyze Changes**
Before creating a PR, review all changes in your branch and categorize them.
**Step 2: Ask User for Confirmation**
Present your analysis to the user and ask for confirmation:
```
Based on the changes in this branch, I've identified:
- [List key changes]
I recommend a [PATCH/MINOR/MAJOR] version bump because [reasoning].
Current version: X.Y.Z
Proposed version: X.Y.Z
Does this classification seem correct? Should I proceed with this version bump?
```
**Step 3: Apply Version Bump**
The version lives in **four** places — keep them in sync on every bump:
1. `package.json` → `"version"`
2. `src-tauri/Cargo.toml` → `version = "..."`
3. `src-tauri/tauri.conf.json` → `"version"`
4. Display string: the About dialog in `src/components/AboutDialog.tsx`

After updating all four, in the feature branch BEFORE creating PR:
```bash
git add -A && git commit -m "chore: bump version to X.Y.Z"
git push
```
**Step 4: Document in PR**
- Update PR title to include new version (e.g., "Redesign UI to CoreTemp layout (v0.1.0)")
- Mention version bump and reasoning in PR description
- List what changed to justify the bump type
### Decision Tree for AI Agents
**MAJOR bump (X.0.0)** - Use when:
- Removing or renaming public components or APIs
- Changing component props in breaking ways
- Restructuring application architecture
- Changing build output or deployment requirements
- Any change that requires users/developers to modify their code
**MINOR bump (0.X.0)** - Use when:
- Adding new feature or component
- Adding new props or options (backwards compatible)
- Significant enhancement to existing feature
- New user-facing functionality
**PATCH bump (0.0.X)** - Use when:
- Fixing bugs or errors
- Correcting typos in UI text
- Improving error messages or logging
- CSS/styling fixes or adjustments
- Performance optimizations (no API changes)
- Accessibility improvements
- Dependency updates (no breaking changes)
- Security patches
**SKIP version bump** - Use when:
- Updating documentation only (README, comments)
- Modifying AGENTS.md or workflow files
- Changing CI/CD configurations
- Updating .gitignore or similar tooling files
- In these cases, label PR with `version:skip` or similar
### Examples
**PATCH: 0.1.0 → 0.1.1**
- "Fix EBUSY crash in tauri dev file watcher"
- "Correct temperature unit not persisting on restart"
- "Improve tray icon color contrast in light theme"
**MINOR: 0.1.0 → 0.2.0**
- "Add per-core temperature mapping from thermal zones"
- "Add overheat protection with sleep/shutdown actions"
- "Add Windows Taskbar temperature display"
**MAJOR: 0.1.x → 1.0.0**
- "Redesign entire UI to CoreTemp layout"
- "Replace PowerShell backend with kernel driver"
- "Change sensor data structure (breaking IPC contract)"
### Integration with Existing Workflow
Version bumping happens IN the feature branch, BEFORE creating the PR:
1. Create feature branch: `git checkout -b issue-X-description`
2. Make your changes and test thoroughly
3. **Analyze changes and ask user about version bump type**
4. **Bump version** in all four locations listed in Step 3
5. Push branch
6. Create PR with version noted in title/description
7. User reviews and merges to main
### Verification Commands
```bash
# Check current version
grep '"version"' package.json
# Check all version locations are in sync
grep -rn "0\.1\.0" package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json src/components/AboutDialog.tsx
# Check if tags exist
git tag -l | tail -5
```
### Human Override
Users can always override AI agent version decisions:
- Manually edit the version files
- Document reasoning in PR comments
- AI agents should defer to user judgment when corrected
## Architecture
- **Frontend**: React 18 with TypeScript, built with Vite, served in the Tauri WebView2
- **Styling**: CSS custom properties drive theming (dark/light), tokens in `src/app/theme.ts`
- **Backend**: Rust (Tauri 2), native ARM64 binary
- **Sensors**: Real telemetry via the native Windows PDH API (ACPI thermal zones, per-core load, per-core-clock-derived Speed); CPU identity via the registry + `GetSystemInfo` (`sensors/identity.rs`), matched to a chip profile via layered detection in `sensors/chips.rs::match_profile` (SKU token → Snapdragon-X family/subfamily from name or Identifier → inferred SKU from core count + rated clock → honest X-family fallback → known non-X vendor/family fallback [legacy Kryo-based Qualcomm chips, Broadcom, MediaTek, NVIDIA] → honest vendor-derived/generic fallback) — see `SENSORS.md` §6. Per-core "P"/"E" badges use `ChipProfile::tier_badge`'s per-family vocabulary, not a hardcoded label — see the real-data contract below.
- **State Management**: React hooks (useState, useEffect, useCallback)
- **Tray**: Exactly one tray icon, always (do NOT re-add the `app.trayIcon` block to `tauri.conf.json` — it duplicates the runtime-built icon). It always shows the single honest CPU temperature — there is no per-core sensor to drive a per-core/mode tray menu. The number is rendered with the native system font via GDI (`sensors/tray_render.rs`, Windows-only, behind a small `tray_icon_size()`/`render_number_rgba()` seam so a macOS menu-bar or Linux tray backend can implement the same two functions later).
- **Settings Persistence**: Tauri store plugin (JSON in app data folder)
## Key directories
- `src/app/` - TypeScript types, theme tokens, hooks (useSettings, useSensors)
- `src/components/` - React components (MenuBar, ProcessorInfo, TempTable, SettingsDialog, OverheatDialog, AboutDialog, MiniMode)
- `src-tauri/src/` - Rust backend (lib.rs = app wiring, sensors/ = telemetry)
- `src-tauri/src/sensors/` - Sensor providers (pdh.rs = primary, identity.rs = raw CPU identity signals from the registry/topology, chips.rs = profiles + detection, tray.rs = color/mode logic, tray_render.rs = Windows GDI digit rendering, types.rs = data shapes)
- `tools/` - Phase 0 sensor probe scripts (PowerShell) + icon generator
## Real-data contract (CRITICAL)
- **All telemetry must be REAL.** No simulated, random, or fallback values anywhere.
- Where a real sensor source is missing (e.g. per-core voltage, true per-core temps), show `—` honestly. Never fabricate a number.
- The working data source on Snapdragon X is the `Thermal Zone Information` PDH counter object (ACPI thermal zones). See `SENSORS.md`.
- There is no true per-core temperature sensor on this firmware. ARMtemp does NOT invent per-core temperatures by mapping zones to cores — it shows one honest CPU temperature (the hottest valid zone, with session Min/Max/Avg) plus genuinely per-core LOAD. This is documented in the About tab / README; do not imply per-core temperature sensing anywhere in the UI.
- Package power is genuinely unavailable from userspace on this firmware (confirmed empty PDH/WMI power counter) — show `—`, do not wire up a fake value.
- Read counters natively via PDH (`sensors/pdh.rs`). Do NOT reintroduce the `wmi` crate's COM/`IWbemServices` query path — it fails with `WBEM_E_NOT_FOUND` under Tauri (see `SENSORS.md` §7 for that history).
- **Core-tier badges must never misrepresent real silicon.** Never hardcode `kind === "efficiency" ? "E" : "P"` in the frontend — always use the per-core `kind_label`/`kind_title` the backend computes from `ChipProfile::tier_badge`. Snapdragon X1 has no efficiency cores at all (badge all "P"); Snapdragon X2's own vocabulary is "Prime"/"Performance" (badge "P"/"P2"), never "Efficiency"; only genuinely hybrid chips (legacy Kryo-based Qualcomm, other real big.LITTLE designs) get an honest "P"/"E". The two-tier `CoreKind` grouping itself (from real OS `EfficiencyClass` topology) stays accurate and is never invented — only the *display vocabulary* is per-family.
- **Detection covers non-Qualcomm boards too** (Raspberry Pi under community Windows-on-ARM builds, etc.) — the app's chip table and vendor fallbacks are not Snapdragon-only. A vendor/model this app doesn't recognize must still get an honest, vendor-derived label (e.g. "Broadcom CPU") — never "Snapdragon Generic" for a chip that isn't a Snapdragon.
## Main window layout structure
The application displays all sensor information in a single window matching Core Temp's actual layout:
1. **Native title bar** — the OS draws it (icon, title, minimize/close); the window is decorated and opaque (no custom chrome, no transparency/blur). Dark/light native chrome follows the app theme via `getCurrentWindow().setTheme()`.
2. **Menu Bar** (`src/components/MenuBar.tsx`) — File (Exit) / Options (Settings, Overheat protection, Toggle Mini Mode, Always on top) / Tools (Refresh sensors, Copy detection report) / Help (About ARMtemp). Rendered as themed HTML dropdowns (a native HMENU doesn't follow dark/light mode on Windows) styled to look like real Win32 menus. No unit toggle — Fahrenheit lives in Settings → Display. Launch flags `--settings` / `--overheat` / `--about` deep-link the dialogs. "Copy detection report" (`get_detection_report` command) copies a plain-text dump of every raw CPU identity signal plus how the chip was matched, for diagnosing misdetections on machines the maintainer doesn't own (see issue #2).
3. **Select CPU** row (combo + `[N] Core(s) [N] Thread(s)` sunken count boxes) + **Processor Information** group box (Win32 etched border, sunken read-only value fields): Model / Platform / Frequency / CPUID full rows; `Boost | Lithography` and `Throttle | TDP` pairs. VID and Revision are intentionally omitted (permanently unavailable on Snapdragon X); Throttle is the live ACPI passive-limit status (red "Yes" while the firmware throttles). CPUID shows the real registry `Identifier` string (e.g. "ARMv8 (64-bit) Family 8 Model 2 Revision 201"), not a repeat of the Model field's marketing string.
4. **Temperature Readings** group box — Tj. Max row, one **CPU Temp** row (Cur. | Min. | Max. | Avg., **colored temperature text** — the app's single honest CPU temperature), then per-core rows (Core # | Load | Min. | Max. | Avg., plain text — genuinely per-core).
5. **Status Bar** — thin native strip with CPU Temp / Low / High (session, since app start).
6. **Mini-mode** drops native decorations at runtime (`setDecorations(false)` + `setSize()`) for a compact always-on-top box, and restores them on exit — matches Core Temp's mini mode.
## Development workflow
1. **ALWAYS** create a new branch for each issue: `git checkout -b issue-{number}-description`
2. Work on features in the branch, commit changes with descriptive messages
3. Create a Pull Request when work is complete (DO NOT MERGE - user handles merges)
4. All new components should be TypeScript with proper interfaces
5. Use CSS custom properties for styling, theme tokens in `src/app/theme.ts`
6. Sensor data flows from Rust `sensors/` → Tauri events → React `useSensors` hook
7. Settings flow from React `useSettings` hook → Tauri store plugin + `update_settings` IPC command
8. Error handling should show honest "—" for missing data, never fabricated values
9. **IMPORTANT**: Always check if dev server is running before starting another one
## Branch and PR workflow
- **Branch naming**: `issue-{number}-brief-description` (e.g., `issue-5-coretemp-redesign`)
- **Commit messages**: Include issue number and clear description (e.g., "Fix issue #5: Implement CoreTemp-style temperature table")
- **PR creation**: Create PR from branch to main, reference issue number in description
- **DO NOT MERGE**: User handles all PR merges
- **Local testing**: After completing changes, always offer to run locally to test changes
- **Dev server**: Never forcefully start dev server - provide clear instructions for user to start it
## Code style
- TypeScript strict mode enabled
- Functional components with hooks (no default exports except root `App`)
- Rust edition 2021, standard `cargo fmt` / `cargo clippy` conventions
- Single quotes, trailing commas where appropriate (TypeScript)
- Component files use PascalCase naming
- Module-per-concern structure under `src-tauri/src/sensors/`
- Commits: conventional-commits style (`feat:`, `fix:`, `docs:`, `chore:`)
- Match the surrounding code's naming and density — don't over-comment
## Sensor backend notes
- **Primary backend**: `sensors/pdh.rs` — native Windows PDH (`pdh.dll`) counters on a dedicated worker thread (PDH handles aren't safely shared across threads). No subprocess, no COM/WMI.
- **`% Processor Time`/`Processor Frequency` are rate counters**: they need two `PdhCollectQueryData` calls before the value is valid; the query is primed once at open so the first real tick already has valid data.
- **`Processor Information` instance names are `group,core`** (e.g. `0,3`) plus `group,_Total`/bare `_Total` roll-ups — parse the trailing index, skip totals. Differs from the flat `0`.. `9` index the old WMI-based path used.
- **PowerShell `$_` mangling** (only relevant to the historical `tools/*.ps1` probe scripts, not the app itself): running PowerShell via `bash -c "powershell -Command '...$_...'"` corrupts variables. Invoke those scripts with `-File`.
- Polling interval default: 1500ms (configurable in Settings → General)
## Testing
- Manual testing required for sensor data (real hardware only — runs on Snapdragon X machines)
- Test temperature unit toggle (°C/°F) updates everywhere (table, status bar, tray)
- Test the tray icon shows the CPU temperature and updates live
- Test settings persistence across restart
- Test close-to-tray behavior
- Test overheat protection notifications
- Test all 3 layouts (Classic, Cards, Dashboard)
## Build and deployment
- Native target: **Windows ARM64** (`aarch64-pc-windows-msvc`). The binary is genuinely ARM64 — no x64 emulation.
- Installer: NSIS only (as of 0.4.4 — MSI was dropped), produced by `npm run tauri build`, named `ARMtemp_<version>_arm64-setup.exe`. Uses a custom template forked from tauri-bundler ([src-tauri/windows/installer.nsi](src-tauri/windows/installer.nsi) — see its header comment for the full delta list and re-sync procedure if the Tauri CLI is upgraded).
- The frontend builds to `dist/`, which Tauri bundles into the native binary.
- Release profile: optimized (`lto`, `strip`, `opt-level = "s"`).
- **In-place upgrades**: the installer upgrades an existing install in place as long as `identifier` (`com.armtemp.app`) and `productName` (`ARMtemp`) in `tauri.conf.json` stay constant across releases — do not change either casually. A prior MSI-based install (≤0.4.3) is detected and migrated automatically (passively, no msiexec prompts). No in-app auto-updater is wired up (no `tauri-plugin-updater`, signing keys, or hosted `latest.json`) — that's a future workstream requiring CI/hosting.
## Common pitfalls
- **EBUSY on `npm run tauri dev`:** Vite's file-watcher must not recurse into `src-tauri/target/` (locked `.dll` during cargo builds). It's excluded in `vite.config.ts` `server.watch.ignored` — don't remove that.
- **PowerShell `$_` mangling:** running PowerShell via bash `-Command` corrupts `$_`/`$var`. Use `-File` with script files in `tools/`. (Only matters for the historical probe scripts — the app no longer shells out to PowerShell at all.)
- **COM/WMI under Tauri:** do not reintroduce the `wmi` crate's `IWbemServices` query path — it fails with `WBEM_E_NOT_FOUND`. PDH (`sensors/pdh.rs`) is the working native approach and doesn't touch COM.
- **Don't wire `docs/claude-design-output/` to live data** — it's a simulated mockup. Port its visuals only.
- **Version sync:** the version must match in all four locations (package.json, Cargo.toml, tauri.conf.json, display strings). See Version Bumping Protocol.
