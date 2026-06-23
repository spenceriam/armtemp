# AGENTS.md
## Setup commands
- Install dependencies: `npm install`
- Start development server: `npm run tauri dev` (starts Vite on `127.0.0.1:1420` + the native Rust backend with hot-reload; check if already running)
- Build for production: `npm run tauri build` (outputs native ARM64 `.exe` + MSI/NSIS installers to `src-tauri/target/release/`)
- Run frontend-only type checking + build: `npm run build`
- Run Rust check (no codegen): `cargo check --manifest-path src-tauri/Cargo.toml`
- Run linter: `cargo clippy --manifest-path src-tauri/Cargo.toml`
## Project overview
ARMTEMP is a native temperature monitor for Snapdragon X / X2 processors (Qualcomm Oryon) on Windows on ARM (ARM64). Built with Tauri 2 (Rust backend) + React 18 (TypeScript frontend), it reads real on-die thermal sensors via ACPI thermal zones and displays per-core temperatures, loads, and power. The app recreates the Core Temp experience for the Snapdragon X family, with a live system-tray icon, mini-mode, overheat protection, and a 6-tab settings dialog. All telemetry is strictly real — no simulated or fallback values anywhere.
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
4. Display strings: `src/App.tsx` title-bar `version=` prop and the About tab in `src/components/SettingsDialog.tsx`

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
grep -rn "0\.1\.0" package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json src/App.tsx src/components/SettingsDialog.tsx
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
- **Sensors**: Real telemetry via PowerShell `Get-CimInstance` (ACPI thermal zones, per-core load, CPU identity)
- **State Management**: React hooks (useState, useEffect, useCallback)
- **Tray**: Tauri tray-icon plugin, live-updating temperature icon
- **Settings Persistence**: Tauri store plugin (JSON in app data folder)
## Key directories
- `src/app/` - TypeScript types, theme tokens, hooks (useSettings, useSensors)
- `src/components/` - React components (TitleBar, ProcessorInfo, TempTable, SettingsDialog, MiniMode)
- `src-tauri/src/` - Rust backend (lib.rs = app wiring, sensors/ = telemetry)
- `src-tauri/src/sensors/` - Sensor providers (powershell.rs = primary, chips.rs = profiles, tray.rs = icon rendering, types.rs = data shapes)
- `tools/` - Phase 0 sensor probe scripts (PowerShell) + icon generator
## Real-data contract (CRITICAL)
- **All telemetry must be REAL.** No simulated, random, or fallback values anywhere.
- Where a real sensor source is missing (e.g. per-core voltage, true per-core temps), show `—` honestly. Never fabricate a number.
- The working data source on Snapdragon X is `Win32_PerfFormattedData_Counters_ThermalZoneInformation` (ACPI thermal zones). See `SENSORS.md`.
- Per-core temps are real *zone* readings mapped to cores (Core #0 = hottest zone), NOT true per-core sensors. Document this in the About tab / README; do not imply otherwise.
- The Rust `wmi` crate's COM path fails with `WBEM_E_NOT_FOUND` under Tauri — use the PowerShell/`Get-CimInstance` backend (`sensors/powershell.rs`). Do NOT reintroduce the COM query path in the main process.
## Main window layout structure
The application displays all sensor information in a single window with a classic Core Temp layout:
1. **Title Bar** - App icon, name + version, window controls (minimize/close). Drag region for frameless window.
2. **Menu Bar** - Tools, Options, Help (no unit toggle — unit lives in Settings).
3. **Processor Information** - Dense 2-column field/value grid (Processor, Platform, Vendor ID, CPUID, Cores, Threads, Frequency, Tj. Max, Load, Power).
4. **Temperature Readings** - Per-core table (Core # | Temp | Low | High | Load) with color-coded temp dots and load bars.
5. **Status Bar** - Sunken footer with CPU Temp / Avg / Low / High.
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
- **Primary backend**: `sensors/powershell.rs` — shells out to `Get-CimInstance`, parses JSON. This is the PROVEN path (Phase 0).
- **Excluded backend**: `sensors/provider.rs` (COM/IWbemServices) — fails with WBEM_E_NOT_FOUND under Tauri. Retained as reference, excluded from build.
- **PowerShell `$_` mangling**: running PowerShell via `bash -c "powershell -Command '...$_...'"` corrupts variables. Put probe scripts in `tools/*.ps1` and invoke with `-File`.
- Polling interval default: 1500ms (configurable in Settings → General)
## Testing
- Manual testing required for sensor data (real hardware only — runs on Snapdragon X machines)
- Test temperature unit toggle (°C/°F) updates everywhere (table, status bar, tray)
- Test tray icon modes (average, highest, all cores, package)
- Test settings persistence across restart
- Test close-to-tray behavior
- Test overheat protection notifications
- Test all 3 layouts (Classic, Cards, Dashboard)
## Build and deployment
- Native target: **Windows ARM64** (`aarch64-pc-windows-msvc`). The binary is genuinely ARM64 — no x64 emulation.
- Installers: MSI + NSIS, produced by `npm run tauri build`, named `ARMTEMP_<version>_arm64_*.msi` / `-setup.exe`.
- The frontend builds to `dist/`, which Tauri bundles into the native binary.
- Release profile: optimized (`lto`, `strip`, `opt-level = "s"`).
## Common pitfalls
- **EBUSY on `npm run tauri dev`:** Vite's file-watcher must not recurse into `src-tauri/target/` (locked `.dll` during cargo builds). It's excluded in `vite.config.ts` `server.watch.ignored` — don't remove that.
- **PowerShell `$_` mangling:** running PowerShell via bash `-Command` corrupts `$_`/`$var`. Use `-File` with script files in `tools/`.
- **COM/WMI under Tauri:** do not reintroduce the `wmi` crate's `IWbemServices` query path — it fails with `WBEM_E_NOT_FOUND`. The PowerShell backend is the working approach.
- **Don't wire `claude-design-output/` to live data** — it's a simulated mockup. Port its visuals only.
- **Version sync:** the version must match in all four locations (package.json, Cargo.toml, tauri.conf.json, display strings). See Version Bumping Protocol.
