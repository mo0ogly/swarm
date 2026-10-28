//! Read a converted or explicitly targeted whole current Work; never release or launch.
use super::{RootStore,RootStoreError as E,preparation_read::PreparationReadObservation};
use crate::{Preparation,Work,ActionPlan,PreparationPlanChange};

/// Readonly comparison of an independently validated draft and the retained
/// conversion spec. Hashes bind observations, not identity, acceptance or a
/// future write. Both Work and preparation revisions still require mutation CAS.
#[derive(serde::Serialize)]
pub struct PreparationRevisionReview {
    pub plan_hash: String,
    pub spec: ActionPlan,
    pub changes: Vec<PreparationPlanChange>,
}
/// Two sequential retained observations, not an atomic snapshot or permission.
/// Both returned revisions must bind any later explicit release or revision. The whole Work
/// is exposed because its current task contracts may differ from the old plan.
#[derive(serde::Serialize)]
pub struct PreparationReleaseReview {
    pub preparation: Preparation,
    pub work: Work,
    #[serde(skip_serializing_if="Option::is_none")]
    pub revision: Option<PreparationRevisionReview>,
}
impl RootStore {
    /// Read a conversion or validated initial append target without changing it.
    /// A changed validated draft additionally yields its current policy-checked
    /// comparison; settings are a third sequential observation, not a snapshot.
    /// `id` is validated by `preparation_show`, whose guarded SQL/method/policy
    /// projection precedes the current `read_work`. An initial target requires
    /// a current validated plan and explicit nonempty Work association. A
    /// mismatched Work association refuses. SQL/codec/path/cleanup failures
    /// propagate and every observed method cause survives a later refusal or
    /// consumer encoding/output/close failure. Two reads are not atomic; the
    /// mutation must check both revisions again. No source text is added to
    /// diagnostics, policy adopted, task released or provider called. A successful
    /// review neither proves acceptance nor authorizes an agent to start.
    pub fn preparation_release_review(&self,id:&str)->Result<PreparationReadObservation<PreparationReleaseReview>,E>{
        let observed=self.preparation_show(id)?;
        let preparation=observed.value;
        let causes=observed.causes;
        let result=(||{
            if let Some(conversion)=preparation.conversion.as_ref() {
                if conversion.work_id!=preparation.work_id{return Err(E::Refused("preparation-release-scope-unported"));}
            } else if preparation.work_id.is_empty() || !preparation.plan_ready {
                return Err(E::Refused("preparation-stale-plan"));
            }
            let work=self.read_work(&preparation.work_id)?;
            let revision=if let Some(conversion)=preparation.conversion.as_ref() {
                let plan=preparation.documents.as_ref().and_then(|d|d.get("plan"));
                if preparation.plan_ready && plan.is_some_and(|d|d.sha256!=conversion.plan_hash) {
                    let plan=plan.ok_or(E::Refused("preparation-stale-plan"))?;
                    let settings=self.settings()?;
                    let limits=settings.values.mutation_limits().map_err(E::Settings)?;
                    let spec=super::preparation_plan::validate(plan.text.as_bytes(),&preparation.method,&settings.values.preparation_plan_policy().map_err(E::Settings)?,&limits.task)?;
                    Some(PreparationRevisionReview{plan_hash:plan.sha256.clone(),changes:crate::preparation_plan_changes::changes(&conversion.spec,&spec),spec})
                }else{None}
            }else{None};
            Ok(PreparationReleaseReview{preparation,work,revision})
        })();
        match result{
            Ok(value)=>Ok(PreparationReadObservation{value,causes}),
            Err(mut primary)=>{
                for cause in causes{primary=E::Cleanup{primary:Some(Box::new(primary)),cleanup:Box::new(cause)};}
                Err(primary)
            }
        }
    }
}
