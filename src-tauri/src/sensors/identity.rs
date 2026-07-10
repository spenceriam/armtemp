//! Real CPU identity signals gathered once from the registry + OS topology.
//! No WMI/COM — plain `RegGetValueW` reads (see `pdh.rs`'s header for why WMI
//! is avoided in this process). This is the raw evidence `chips::match_profile`
//! reasons from; nothing here is fabricated or guessed.
//!
//! Why more than just `ProcessorNameString`: issue #2 showed that some OEM
//! firmware (Surface Laptop 8, Snapdragon X2 Elite) reports a bare marketing
//! name with no SKU token (`X2E78100`) in it at all, so name-substring
//! matching alone cannot identify the exact part. CPU-Z/HWiNFO manage it by
//! also reading the per-core `~MHz` registry value (each core's cluster max
//! clock) and the real P/E core topology — both gathered here.

use windows::core::HSTRING;
use windows::Win32::System::Registry::{
    RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
};

const ERROR_SUCCESS: u32 = 0;

/// Every real, non-fabricated identity signal available for the detected CPU.
#[derive(Clone, Debug, Default)]
pub struct CpuIdentity {
    /// `ProcessorNameString` — e.g. "Snapdragon(R) X2 Elite" or
    /// "Snapdragon(R) X 10-core X1P64100 @ 3.40 GHz". May lack a SKU token.
    pub name: Option<String>,
    /// `Identifier` — e.g. "ARMv8 (64-bit) Family 8 Model 2 Revision 201".
    /// "Model" is the MIDR part number in hex: 0x001 = Oryon (X1 family),
    /// 0x002 = Oryon V3 (X2 family) — a name-independent family signal.
    pub identifier: Option<String>,
    /// `VendorIdentifier` — e.g. "Qualcomm Technologies Inc".
    pub vendor: Option<String>,
    /// Per-logical-core `~MHz` value (each core's cluster max/rated clock;
    /// this is what CPU-Z's "Original Processor Frequency" and HWiNFO show).
    pub per_core_mhz: Vec<u32>,
    /// Logical processor count (`GetSystemInfo`).
    pub logical_cores: u32,
    /// Real Performance-core count from OS topology, when available.
    pub perf_cores: Option<u32>,
    /// Real Efficiency-core count from OS topology, when available.
    pub eff_cores: Option<u32>,
}

impl CpuIdentity {
    /// The highest per-core `~MHz` seen — the chip's real rated boost clock,
    /// independent of what the name string says.
    pub fn max_mhz(&self) -> Option<u32> {
        self.per_core_mhz.iter().copied().max()
    }

    /// Rated clock embedded in `ProcessorNameString`, e.g. "@ 4.03 GHz" ->
    /// Some(4030). Parsed from the RAW name — `normalized_name()` strips the
    /// '.'/'@' this needs. Issue #2: some OEM firmware reports boot-time
    /// cluster clocks via per-core `~MHz`, not the rated boost, so this is a
    /// second, independent clock signal `infer_within_family` can fall back
    /// to (or prefer).
    pub fn name_clock_mhz(&self) -> Option<u32> {
        let lower = self.name_lower();
        let unit_pos = [lower.rfind("ghz"), lower.rfind("mhz")].into_iter().flatten().max()?;
        let is_ghz = lower[unit_pos..].starts_with("ghz");
        let digits: String = lower[..unit_pos]
            .trim_end()
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if digits.is_empty() {
            return None;
        }
        let value: f64 = digits.parse().ok()?;
        let mhz = if is_ghz { (value * 1000.0).round() } else { value.round() } as u32;
        (500..=8000).contains(&mhz).then_some(mhz)
    }

    /// MIDR part number decoded from `Identifier`'s "Model <hex>" token.
    /// `None` if `identifier` is absent or doesn't parse. `1` = Oryon (X1
    /// family), `2` = Oryon V3 (X2 family) — see the Linux kernel's
    /// `cputype.h` / pytorch/cpuinfo's ARM uarch table for the same mapping.
    pub fn oryon_generation(&self) -> Option<u32> {
        let id = self.identifier.as_ref()?;
        let after = id.split("Model ").nth(1)?;
        let token = after.split_whitespace().next()?;
        u32::from_str_radix(token, 16).ok()
    }

    pub fn name_lower(&self) -> String {
        self.name.as_deref().unwrap_or("").to_lowercase()
    }

    /// Keep only ASCII alphanumerics, lowercased — immune to hyphens, spaces,
    /// and "(R)"/"(TM)" marks that vary across OEM firmware.
    pub fn normalized_name(&self) -> String {
        self.name_lower()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect()
    }
}

fn read_reg_sz(subkey: &str, value: &str) -> Option<String> {
    unsafe {
        let hsubkey = HSTRING::from(subkey);
        let hvalue = HSTRING::from(value);
        let mut size: u32 = 0;
        let st = RegGetValueW(HKEY_LOCAL_MACHINE, &hsubkey, &hvalue, RRF_RT_REG_SZ, None, None, Some(&mut size));
        if st.0 != ERROR_SUCCESS || size == 0 {
            return None;
        }
        let mut buf: Vec<u16> = vec![0u16; (size as usize).div_ceil(2)];
        let st2 = RegGetValueW(
            HKEY_LOCAL_MACHINE,
            &hsubkey,
            &hvalue,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            Some(&mut size),
        );
        if st2.0 != ERROR_SUCCESS {
            return None;
        }
        let s = String::from_utf16_lossy(&buf);
        Some(s.trim_end_matches('\0').trim().to_string())
    }
}

fn read_reg_dword(subkey: &str, value: &str) -> Option<u32> {
    unsafe {
        let hsubkey = HSTRING::from(subkey);
        let hvalue = HSTRING::from(value);
        let mut data: u32 = 0;
        let mut size: u32 = std::mem::size_of::<u32>() as u32;
        let st = RegGetValueW(
            HKEY_LOCAL_MACHINE,
            &hsubkey,
            &hvalue,
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut core::ffi::c_void),
            Some(&mut size),
        );
        if st.0 == ERROR_SUCCESS {
            Some(data)
        } else {
            None
        }
    }
}

/// Gather every identity signal for logical core 0..`logical_cores`.
/// `perf_cores`/`eff_cores` come from the OS topology query already done in
/// `pdh.rs` (`query_core_efficiency_classes`) — passed in rather than
/// re-queried here to keep this module a pure registry reader.
pub fn gather(logical_cores: u32, perf_cores: Option<u32>, eff_cores: Option<u32>) -> CpuIdentity {
    let base = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";
    let name = read_reg_sz(base, "ProcessorNameString");
    let identifier = read_reg_sz(base, "Identifier");
    let vendor = read_reg_sz(base, "VendorIdentifier");

    let mut per_core_mhz = Vec::with_capacity(logical_cores as usize);
    for i in 0..logical_cores {
        let subkey = format!(r"HARDWARE\DESCRIPTION\System\CentralProcessor\{i}");
        if let Some(mhz) = read_reg_dword(&subkey, "~MHz") {
            per_core_mhz.push(mhz);
        }
    }

    CpuIdentity {
        name,
        identifier,
        vendor,
        per_core_mhz,
        logical_cores,
        perf_cores,
        eff_cores,
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn oryon_generation_parses_model_1_and_2() {
        let mut id = CpuIdentity::default();
        id.identifier = Some("ARMv8 (64-bit) Family 8 Model 1 Revision 201".into());
        assert_eq!(id.oryon_generation(), Some(1));
        id.identifier = Some("ARMv8 (64-bit) Family 8 Model 2 Revision 201".into());
        assert_eq!(id.oryon_generation(), Some(2));
    }

    #[test]
    fn oryon_generation_none_when_malformed_or_absent() {
        let mut id = CpuIdentity::default();
        assert_eq!(id.oryon_generation(), None);
        id.identifier = Some("garbage string".into());
        assert_eq!(id.oryon_generation(), None);
    }

    #[test]
    fn max_mhz_is_the_highest_per_core_reading() {
        let mut id = CpuIdentity::default();
        id.per_core_mhz = vec![2976, 3418, 710];
        assert_eq!(id.max_mhz(), Some(3418));
    }

    #[test]
    fn normalized_name_strips_marks_and_lowercases() {
        let mut id = CpuIdentity::default();
        id.name = Some("Snapdragon(R) X2 Elite - X2E-78-100".into());
        assert_eq!(id.normalized_name(), "snapdragonrx2elitex2e78100");
    }

    #[test]
    fn name_clock_mhz_parses_x2_bare_marketing_name() {
        let mut id = CpuIdentity::default();
        id.name = Some("Snapdragon X2 Elite @ 4.03 GHz".into());
        assert_eq!(id.name_clock_mhz(), Some(4030));
    }

    #[test]
    fn name_clock_mhz_ignores_sku_digits_and_reads_trailing_clock() {
        let mut id = CpuIdentity::default();
        id.name = Some("Snapdragon(R) X Elite - X1E-78-100 - Qualcomm(R) Oryon(TM) CPU @ 3.40 GHz".into());
        assert_eq!(id.name_clock_mhz(), Some(3400));
    }

    #[test]
    fn name_clock_mhz_parses_mhz_unit() {
        let mut id = CpuIdentity::default();
        id.name = Some("Snapdragon X Plus - X1P64100 @ 3400 MHz".into());
        assert_eq!(id.name_clock_mhz(), Some(3400));
    }

    #[test]
    fn name_clock_mhz_none_when_absent_or_garbage() {
        let mut id = CpuIdentity::default();
        assert_eq!(id.name_clock_mhz(), None);
        id.name = Some("Snapdragon X2 Elite".into());
        assert_eq!(id.name_clock_mhz(), None);
        id.name = Some("Snapdragon X2 Elite @ GHz".into());
        assert_eq!(id.name_clock_mhz(), None);
    }
}
