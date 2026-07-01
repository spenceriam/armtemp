//! Sensor subsystem: real telemetry backends for Snapdragon X.
pub mod chips;
pub mod pdh;
pub mod tray;
pub mod types;

pub use chips::ChipProfile;
pub use pdh::PdhProvider;
pub use types::SensorSnapshot;
