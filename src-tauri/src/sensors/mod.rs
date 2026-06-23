//! Sensor subsystem: real telemetry backends for Snapdragon X.
pub mod chips;
pub mod powershell;
pub mod tray;
pub mod types;

// NOTE: `provider` (the COM/IWbemServices backend) is excluded from the build —
// it fails with WBEM_E_NOT_FOUND when called from inside a Tauri process. The
// PowerShell-backed `powershell` module is the working primary backend. The
// COM path is retained as a future option (e.g. a dedicated subprocess).
#[allow(unused)]
#[path = "provider.rs"]
mod provider_unused;

pub use chips::ChipProfile;
pub use powershell::PowerShellProvider;
pub use types::SensorSnapshot;
