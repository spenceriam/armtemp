// Reproduce the app scenario: WMI from a spawned worker, as a WINDOWS-SUBSYSTEM
// (GUI) binary (no console), writing results to a log file.
#![windows_subsystem = "windows"]
use serde::Deserialize;
use std::io::Write;
use std::sync::mpsc;
use std::thread;
use wmi::{COMLibrary, WMIConnection};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct ProcessorRow {
    name: String,
    #[serde(rename = "NumberOfCores")]
    cores: Option<u32>,
}

fn main() {
    let mut log = std::fs::File::create("C:/Users/spenc/GitHub/armtemp/wmitest-gui.log").unwrap();
    writeln!(log, "GUI-subsystem WMI test").unwrap();

    // Spawn the worker exactly like the app does.
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let r = (|| -> Result<String, String> {
            let com = COMLibrary::new().map_err(|e| format!("COM new: {e:?}"))?;
            let conn = WMIConnection::new(com).map_err(|e| format!("conn new: {e:?}"))?;
            let v: Vec<ProcessorRow> = conn
                .raw_query("SELECT Name, NumberOfCores FROM Win32_Processor")
                .map_err(|e| format!("query: {e:?}"))?;
            Ok(format!("OK: {:?}", v.first()))
        })();
        let _ = tx.send(r.unwrap_or_else(|e| e));
    });

    match rx.recv() {
        Ok(msg) => writeln!(log, "worker result: {msg}").unwrap(),
        Err(e) => writeln!(log, "worker channel died: {e}").unwrap(),
    }
}
