//! Explicit display and complete-verification policies; neither is proof authority.
use super::{refuse, size, EngineSettingsError, EngineSettingsValues, SettingCap};
/// Current adopted report-reader byte choices. Preview limits retained text;
/// verification limits complete private report hashing, never truncated evidence.
/// None explicitly removes only the corresponding cap. No IO or rights follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportReadPolicy {
    pub preview_max_bytes: Option<usize>,
    pub verification_max_bytes: Option<usize>,
}
impl EngineSettingsValues {
    /// Read both explicitly adopted report caps. Missing/partial groups refuse;
    /// null is unlimited, zero retains no preview bytes or rejects a nonempty
    /// complete verification respectively. Negative/noncanonical counts fail the
    /// authoritative decoder; platform overflow remains a typed conversion error.
    /// This independent reading policy adopts no unrelated execution defaults,
    /// performs no IO and grants no path, provider, review or lifecycle authority.
    pub fn report_read_policy(&self) -> Result<ReportReadPolicy, EngineSettingsError> {
        let cap = |v: Option<SettingCap>| {
            let SettingCap(value) = v.ok_or_else(|| refuse("settings-report-read-policy-not-configured"))?;
            value.map(|n| size(Some(n))).transpose()
        };
        Ok(ReportReadPolicy {
            preview_max_bytes: cap(self.report_preview_max_bytes)?,
            verification_max_bytes: cap(self.report_verification_max_bytes)?,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_read_caps_absence_null_zero_and_invalid_inputs() {
        let mut values = EngineSettingsValues::installation_defaults().unwrap();
        assert_eq!(values.schema_version(),49);
        // Isolate the historical report-read group from the later independent
        // cockpit and ledger-root choices; removing report caps still exposes v46.
        values.cockpit_default_autonomy = None;
        values.paea_settlement_ledger_root = None;
        assert_eq!(values.schema_version(),47);
        assert_eq!(values.report_read_policy().unwrap().preview_max_bytes,Some(262144));
        values.report_preview_max_bytes=None;
        assert_eq!(values.report_read_policy().unwrap_err().reason(),"settings-report-read-policy-not-configured");
        assert!(values.validate().is_err());
        values.report_verification_max_bytes=None;
        values.validate().unwrap();
        assert_eq!(values.schema_version(),46);
        values.report_preview_max_bytes=Some(SettingCap(None));
        values.report_verification_max_bytes=Some(SettingCap(Some(super::super::SettingCount(0))));
        assert_eq!(values.report_read_policy().unwrap(),ReportReadPolicy{preview_max_bytes:None,verification_max_bytes:Some(0)});
        let raw=serde_json::to_value(&values).unwrap();
        assert!(raw["report_preview_max_bytes"].is_null());
        for bad in [serde_json::json!("-1"),serde_json::json!(1),serde_json::json!("01")] {
            let mut altered=raw.clone();altered["report_preview_max_bytes"]=bad;
            assert!(EngineSettingsValues::decode(&serde_json::to_vec(&altered).unwrap()).is_err());
        }
    }
    #[test]
    fn report_read_catalogue_matches_persisted_choices() {
        let catalog=crate::EngineSettingsCatalog::installation().unwrap();
        let raw=serde_json::to_value(catalog).unwrap();
        for key in ["report_preview_max_bytes","report_verification_max_bytes"] {
            let field=raw["fields"].as_array().unwrap().iter().find(|f|f["name"]==key).unwrap();
            assert_eq!(field["unit"],"bytes");assert_eq!(field["nullable"],true);assert_eq!(field["introduced_schema_version"],47);
        }
    }
}
