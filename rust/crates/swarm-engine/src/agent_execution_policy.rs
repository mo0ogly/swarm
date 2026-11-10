//! Explicit persisted supervision settings; no execution authority or defaults.
use crate::{EngineSettingsError, SettingCap, SettingCount};
use serde::{Deserialize, Deserializer, Serialize};
/// Complete v18 policy, validated again before runtime consumption. Historical
/// absence stays distinct at the settings boundary. Capturing bytes/lines is
/// separate from observed tool counters, provider vitality and task acceptance.
/// Null caps explicitly remove that ceiling; there is no Rust fallback value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentExecutionPolicy {
    /// Intervalle strictement positif entre les contrôles de stockage et les heartbeats du superviseur, en millisecondes. Il ne limite pas la durée du fournisseur.
    pub poll_interval_ms: SettingCount,
    /// Âge strictement positif au-delà duquel un heartbeat est signalé périmé. Observation seulement : cela ne prouve pas la mort du processus et ne déclenche pas d’arrêt.
    pub heartbeat_stale_ms: SettingCount,
    /// Délai non négatif avant escalade vers KILL après une demande d’arrêt. Zéro demande une escalade immédiate. Aucun arrêt n’est décidé sur la seule durée du fournisseur.
    pub term_grace_ms: SettingCount,
    /// Attente maximale des flux après fin du leader, en millisecondes. Zéro n’attend pas un flux encore ouvert ; sans plafond peut bloquer indéfiniment. Un drain incomplet reste un incident et ne prouve pas la clôture sûre de la mission.
    #[serde(deserialize_with = "explicit_cap")]
    pub output_drain_ms: SettingCap,
    /// Plafond d’un événement fournisseur. Zéro rend les événements non vides inobservables ; sans plafond retire la limite mémoire de ce canal. Un dépassement dégrade la visibilité, sans arrêter le drainage ni prouver un résultat d’outil.
    #[serde(deserialize_with = "explicit_cap")]
    pub event_max_bytes: SettingCap,
    /// Plafond cumulé de capture brute lorsque la tentative l’active. Zéro ne retient aucun contenu ; sans plafond peut augmenter fortement mémoire et stockage. Le processus et les compteurs continuent après troncature explicite.
    #[serde(deserialize_with = "explicit_cap")]
    pub capture_max_bytes: SettingCap,
    /// Plafond cumulé des lignes de capture brute. Zéro ne retient aucune ligne ; sans plafond retire cette limite. Ce choix ne borne ni les appels d’outils ni les lignes effectivement produites.
    #[serde(deserialize_with = "explicit_cap")]
    pub capture_max_lines: SettingCap,
    /// Plafond cumulé des messages assistant explicitement publics. Zéro ne retient pas leur contenu ; sans plafond retire cette limite. Raisonnement et données d’outils ne deviennent pas des messages publics.
    #[serde(deserialize_with = "explicit_cap")]
    pub public_message_max_bytes: SettingCap,
    /// Plafond UTF-8 d’un détail de journal, après nettoyage. Zéro retire le détail, pas le code d’incident ou l’état bloquant ; sans plafond conserve tout le détail autorisé. La troncature respecte les frontières UTF-8.
    #[serde(deserialize_with = "explicit_cap")]
    pub log_message_max_bytes: SettingCap,
}
/// Require the nullable field to be present: missing must not become null.
/// Supplied values use the shared canonical decimal/nonnegative representation.
fn explicit_cap<'de, D: Deserializer<'de>>(d: D) -> Result<SettingCap, D::Error> {
    SettingCap::deserialize(d)
}
impl AgentExecutionPolicy {
    /// Validate complete supplied/frozen values without IO, normalization or
    /// implicit defaults. Poll/heartbeat are positive, grace/caps nonnegative.
    /// Failure retains a safe family; success is data validation only and grants
    /// no process, storage mutation, provider permission or acceptance.
    pub fn validate(&self) -> Result<(), EngineSettingsError> {
        if self.poll_interval_ms.0 < 1 || self.heartbeat_stale_ms.0 < 1 {
            return Err(EngineSettingsError::Refused(
                "settings-supervision-interval-positive",
            ));
        }
        if self.term_grace_ms.0 < 0
            || [
                self.output_drain_ms,
                self.event_max_bytes,
                self.capture_max_bytes,
                self.capture_max_lines,
                self.public_message_max_bytes,
                self.log_message_max_bytes,
            ]
            .iter()
            .any(|cap| cap.0.is_some_and(|n| n.0 < 0))
        {
            return Err(EngineSettingsError::Refused(
                "settings-supervision-value-nonnegative",
            ));
        }
        Ok(())
    }
}
