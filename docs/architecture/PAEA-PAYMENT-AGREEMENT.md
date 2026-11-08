# Accord séparé exact du règlement fictif — P1/G1

[English](../en/PAEA-PAYMENT-AGREEMENT.md) · [suivi](../plans/actions-confiance/paea/P1-G1-20261103.md)

État : implémentation en qualification, aucun enrôlement humain ni service
d'effet protégé globalement qualifié par cette tranche.

Le réglage existant require_separation conserve sa source persistée Trust,
sa validation et ses interfaces Admin/API/CLI. Aucun nouveau choix opérateur.
Lorsqu'il vaut true, le règlement fictif exige un accord durable Trust séparé
pour la ressource de protocole `paea-payment/<empreinte canonique complète>`.
La chaîne existante mandate-create → request → approve produit cet accord.
Montant, devise et bénéficiaire sont ceux du document canonique et restent
vérifiés indépendamment. L'empreinte inclut version de facture, contexte,
sources et politique ; une modification exige un nouvel accord.

La CLI `paea settlement-call` expose `approval_resource` dans son événement
préparé, sans délivrer de droit. L'opérateur peut préparer les termes exacts
avec les opérations Trust existantes, hors du processus de consommation.
Le contrôle personnel et métier signé reste nécessaire ; cet accord ne le
remplace pas. Pas de création automatique d'identité, mandat ou approbation.

Au dernier usage, sous les réservations projet/Trust conservées, le moteur
vérifie chaîne et originaux, révision courante, action approuvée, acteurs distincts
et rôles courants, demande admise, mandat actif et liaison aux octets canoniques.
Le délai du mandat est recontrôlé avant l'engagement SQLite ; ce n'est pas une
garantie temps réel entre contrôle et COMMIT.

Lorsque la matrice avancée de séparation des rôles est activée par-dessus
require_separation, le moteur recontrôle au point d'engagement les participants
réels de l'action approuvée liée à l'accord — délégataire admin du mandat,
requester, approver, effect — avec la comparaison de groupes du moniteur Trust
existant. Un conflit refuse sans aucune insertion de grand livre
(`paea-settlement-role-separation`) ; un mapping manquant refuse aussi
(`paea-settlement-role-mapping`). Les groupes restent des assertions
d'administrateur et ne prouvent aucune identité humaine. Une matrice activée
sans require_separation reste refusée
(`paea-settlement-attributed-admission-required`), sans désactiver la politique
sauvegardée. La politique de base reste exigée au point d'engagement : un droit
fraîchement signé ne rend pas une politique périmée utilisable.

Le reçu ajoute un champ optionnel agreement : action_id, mandate_id, requester,
approver, action_digest, evidence_hash, original_digest, policy_revision et
expires_at. La CLI conserve cette référence dans son résultat. Ce sont des données
d'audit locales, pas une capacité, une signature externe ou une preuve d'attention
humaine. La lecture refuse les métadonnées d'accord incohérentes. Les reçus anciens
sans agreement restent lisibles et ne prouvent pas de séparation. Un binaire ancien
dont le décodeur refuse les champs inconnus ne doit pas relire ces nouveaux reçus :
conserver la compatibilité des images lors d'une reprise de version.

Accord absent/périmé/révoqué ou contenu changé : refus typé
paea-settlement-attributed-admission-required avant nouvel effet. Corruption des
preuves ou reçus : erreur typée avec cause conservée, jamais un succès vide.
Réconciliation reste une lecture sous autorité dédiée ; elle n'accorde aucun
nouvel engagement. Un effet déjà enregistré n'est pas annulé par révocation.
Les erreurs tardives conservent les observations connues/incertaines existantes.

Les fixtures ne remplacent ni enrôlement humain réel, ni protection des clés et
services contre le domaine agent, ni qualification E3. Les champs de contexte
restent déclarés : les lier exactement ne prouve pas leur provenance.
