//! Read-only quota observation and preview under the saved settings reservation.
use super::{finished, RootStore, RootStoreError as E};
use crate::{QuotaView, SettingCount, WorkEnvironment};
/// Before/proposed values share one Work snapshot and saved policy revision.
/// Proposed revision remains unchanged: preview never publishes or reserves.
#[derive(serde::Serialize)]
pub struct QuotaPreview {
    pub settings_revision: SettingCount,
    pub before: QuotaView,
    pub after: QuotaView,
}
impl RootStore {
    /// Read actual mission limits and usage, preserving absent legacy fields.
    /// Invalid hierarchy/negative usage and storage/codec errors retain causes.
    /// No defaults, migration, provider, ownership or mutation is introduced.
    pub fn quotas(&self, work: &str) -> Result<QuotaView, E> {
        let current = self.read_work(work)?;
        crate::quota_view(&current).map_err(|e| E::Launch(Box::new(e)))
    }
    /// Preview a strict bounded proposal against one Work snapshot and reserved
    /// saved policy. Actual environment supplies unverified local audit actor/time;
    /// request authority is rejected. All apply checks run on a private clone.
    /// Returns before/after or typed validation/storage/cleanup causes, with no
    /// SQL writes or publication. Apply must recheck revision and current policy;
    /// this observation is neither a reservation nor a launch permission.
    pub fn preview_quotas<Ev: WorkEnvironment>(
        &self,
        work: &str,
        raw: &[u8],
        environment: &mut Ev,
    ) -> Result<QuotaPreview, E> {
        crate::check_command_input_size(raw).map_err(E::Input)?;
        let launch = |e| E::Launch(Box::new(e));
        let request = crate::decode_quota_change(raw).map_err(launch)?;
        let reservation = self.directory.reserve(false, false).map_err(E::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            let policy = settings.values.quota_reason_policy().map_err(E::Settings)?;
            let mut current = self.read_work(work)?;
            let before = crate::quota_view(&current).map_err(launch)?;
            let actor = environment.operator().map_err(E::Runtime)?;
            let at = environment.timestamp().map_err(E::Runtime)?;
            crate::apply_quota_change(&mut current, &request, &actor, &at, &policy)
                .map_err(launch)?;
            Ok(QuotaPreview {
                settings_revision: settings.revision,
                before,
                after: crate::quota_view(&current).map_err(launch)?,
            })
        })();
        finished(result, reservation.close().map_err(E::Storage))
    }
}
