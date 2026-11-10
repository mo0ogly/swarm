//! Read-only mission discovery, separate from admission's settings reservation.
use super::{finished, RootStore, RootStoreError as E};
use crate::{MissionControlView, MissionSupervisionPolicy, SettingCount};

/// Recorded settings and one coherent SQL control view, never a lease or grant.
/// File and SQL observations are not one atomic transaction; every later mission
/// admission still reads current rights, policy, budget and physical binding.
pub struct MissionDiscoveryObservation {
    pub settings_revision: SettingCount,
    pub policy: MissionSupervisionPolicy,
    pub missions: Vec<(String, MissionControlView)>,
}

impl RootStore {
    /// Observe the atomically published validated settings and mission controls
    /// without a shared Admin reservation. One SQL read transaction binds each
    /// Work revision to its consent/control/lifecycle rows. Held Root descriptors
    /// and before/after path checks reject replacement; checked close failures
    /// retain primary and cleanup causes. No defaults, retries, writes, process
    /// actions or execution permissions follow. Missing/malformed policy/rows
    /// refuse; the returned observation can become stale immediately.
    pub fn mission_discovery_observation(&self) -> Result<MissionDiscoveryObservation, E> {
        let binding = self.bind_root()?;
        let result = (|| {
            super::material_path::verify_material_path(self,&binding)?;
            let settings = self.read_settings()?;
            let policy = settings.values.mission_supervision_policy().map_err(E::Settings)?;
            let missions = super::mission_supervision_store::read_transaction(&self.db, || {
                use super::mission_controls::{consent,controls,MissionAutonomy};
                use crate::MissionSupervisionError as M;
                let works = crate::list_recorded_works(&self.db).map_err(|e|M::Root(Box::new(E::Work(e))))?;
                let mut missions = Vec::new();
                for work in works {
                    let consent = consent(&self.db,&work.id)?;
                    let controls = controls(&self.db,&work.id)?;
                    let count:i64=super::mission_supervision_store::optional(&self.db,"SELECT count(*) FROM mission_lifecycle WHERE work_id=?",[&work.id],|r|r.get(0))?.ok_or(M::Refused("mission-controls-row-missing"))?;
                    missions.push((work.id,MissionControlView {
                        revision:work.revision,
                        paused:controls.as_ref().map(|c|c.paused),
                        autonomy:controls.as_ref().map(|c|match c.autonomy {MissionAutonomy::Manual=>"manuel",MissionAutonomy::Assisted=>"assiste",MissionAutonomy::Automatic=>"autonome"}),
                        slots:controls.as_ref().map(|c|c.slots),
                        consent_enabled:consent.as_ref().map(|c|c.enabled),archived:count!=0,
                    }));
                }
                Ok(missions)
            }).map_err(|e|E::Mutation(crate::WorkEngineError::Mission(Box::new(e))))?;
            super::material_path::verify_material_path(self,&binding)?;
            if !binding.matches(self)? { return Err(E::Refused("web-root-binding-changed")); }
            Ok(MissionDiscoveryObservation { settings_revision:settings.revision,policy,missions })
        })();
        finished(result,binding.close())
    }
}
