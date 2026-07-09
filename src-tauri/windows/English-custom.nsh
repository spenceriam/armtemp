; ARMtemp custom English strings for the NSIS installer.
;
; Forked from tauri-bundler's stock English.nsh at tag `tauri-cli-v2.11.3`
; (crates/tauri-bundler/src/bundle/windows/nsis/languages/English.nsh).
; `addOrReinstall` is reworded (same-version case is now presented as a
; Repair/Uninstall maintenance choice rather than "Add/Reinstall components").
; `unableToUninstallAutoResolved` and `setupAlreadyRunning` are new strings
; for installer.nsi's edge-case fixes (auto-resolved migration failure, and
; the Setup single-instance guard) — everything else is verbatim, see
; installer.nsi's header comment for the re-sync procedure if the Tauri CLI
; is upgraded.
;
; Note: the "app is running" strings below intentionally keep the literal
; `{{product_name}}` marker (NOT a handlebars placeholder here) — utils.nsh's
; `CheckIfAppIsRunning` macro runtime-replaces that exact marker via
; `nsis_tauri_utils::StrReplace` with the real product name. Do not resolve
; it at file level or that runtime substitution becomes a no-op.

LangString addOrReinstall ${LANG_ENGLISH} "Repair ${PRODUCTNAME}"
LangString alreadyInstalled ${LANG_ENGLISH} "Already Installed"
LangString alreadyInstalledLong ${LANG_ENGLISH} "${PRODUCTNAME} ${VERSION} is already installed. Select the operation you want to perform and click Next to continue."
LangString appRunning ${LANG_ENGLISH} "{{product_name}} is running! Please close it first then try again."
LangString appRunningOkKill ${LANG_ENGLISH} "{{product_name}} is running!$\nClick OK to kill it"
LangString chooseMaintenanceOption ${LANG_ENGLISH} "Choose the maintenance option to perform."
LangString choowHowToInstall ${LANG_ENGLISH} "Choose how you want to install ${PRODUCTNAME}."
LangString createDesktop ${LANG_ENGLISH} "Create desktop shortcut"
LangString dontUninstall ${LANG_ENGLISH} "Do not uninstall"
LangString dontUninstallDowngrade ${LANG_ENGLISH} "Do not uninstall (Downgrading without uninstall is disabled for this installer)"
LangString failedToKillApp ${LANG_ENGLISH} "Failed to kill {{product_name}}. Please close it first then try again"
LangString installingWebview2 ${LANG_ENGLISH} "Installing WebView2..."
LangString newerVersionInstalled ${LANG_ENGLISH} "A newer version of ${PRODUCTNAME} is already installed! It is not recommended that you install an older version. If you really want to install this older version, it's better to uninstall the current version first. Select the operation you want to perform and click Next to continue."
LangString older ${LANG_ENGLISH} "older"
LangString olderOrUnknownVersionInstalled ${LANG_ENGLISH} "An $R4 version of ${PRODUCTNAME} is installed on your system. It's recommended that you uninstall the current version before installing. Select the operation you want to perform and click Next to continue."
LangString setupAlreadyRunning ${LANG_ENGLISH} "${PRODUCTNAME} Setup is already running."
LangString silentDowngrades ${LANG_ENGLISH} "Downgrades are disabled for this installer, can't proceed with the silent installer, please use the graphical interface installer instead.$\n"
LangString unableToUninstall ${LANG_ENGLISH} "Unable to uninstall!"
LangString unableToUninstallAutoResolved ${LANG_ENGLISH} "Setup couldn't remove the previous installation automatically. No changes have been made."
LangString uninstallApp ${LANG_ENGLISH} "Uninstall ${PRODUCTNAME}"
LangString uninstallBeforeInstalling ${LANG_ENGLISH} "Uninstall before installing"
LangString unknown ${LANG_ENGLISH} "unknown"
LangString webview2AbortError ${LANG_ENGLISH} "Failed to install WebView2! The app can't run without it. Try restarting the installer."
LangString webview2DownloadError ${LANG_ENGLISH} "Error: Downloading WebView2 Failed - $0"
LangString webview2DownloadSuccess ${LANG_ENGLISH} "WebView2 bootstrapper downloaded successfully"
LangString webview2Downloading ${LANG_ENGLISH} "Downloading WebView2 bootstrapper..."
LangString webview2InstallError ${LANG_ENGLISH} "Error: Installing WebView2 failed with exit code $1"
LangString webview2InstallSuccess ${LANG_ENGLISH} "WebView2 installed successfully"
LangString deleteAppData ${LANG_ENGLISH} "Delete the application data"
