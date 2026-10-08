# E6 — essai d482c33 et contention SQLite

Essai réel isolé : FAIL. La mission principale demeure non validée.

- Travail : `w-7660e2edd92e17bc25487961` ; moteur d482c33.
- Deux productions réelles (8 et 10 outils), défaut injecté puis correction.
- Une revue consommée ; enregistrement interrompu avec SQLITE_BUSY.
- Le sous-planificateur refuse une troisième production, plafond conservé.
- Clôture non atteinte : cet essai ne prouve pas encore le correctif des événements de clôture.

## Défaut reproduit et correction

Les transactions review.* ne réservaient pas l’écrivain SQLite avant lecture.
Le test existant tentait une écriture autocommit concurrente, qui échoue aussi
sur un verrou de lecture en journal classique : il ne distinguait pas ces cas.
Le test utilise désormais BEGIN IMMEDIATE, qui expose exactement l’absence de
réservation. Trois cas review.managed.claim/result/batch.claim échouent avant
correction. Les transactions review.* réservent maintenant l’écriture avant
lecture, comme le faisait déjà managed.integrated. Aucun appel modèle n’est
rejoué par ce changement. Les gardes de révision/provenance sont conservées.

Preuves locales : e6-apex-events-20260928/observed.json et result.json,
codex-takeover-20260927/apex-e6-review-writer-{race,full}.log.

Validation : suite complète ok  	swarm.local/companion	371.547s, concurrence race2.903sPASS, campagne CLI déterministe5tests21.672sPASS, vet/diffPASS. Aucun nouvel essai IA du correctif.
