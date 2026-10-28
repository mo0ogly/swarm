# M29 — MEM-P1, lecteurs explicites de Work

17 octobre 2026, accord « go » sur les cinq étapes. Codex seul, méthodes
replan/APEX et audit-pdca avec corrections autorisées. Reprise sur M28 ; branche
et sources initiales conservées dans `evidence/m29-20261017/baseline.json`.
Les modifications concurrentes du chantier identité sont exclues.

## Choix et séquence

Les lecteurs explicites reconstruisent parfois un Work depuis le corps seul,
sans concordance avec les colonnes SQL. Deux solutions : recopier les contrôles
dans chaque module, ou réutiliser le noyau M28. Retenir ce noyau ; pour une
jointure, séparer le décodage d'enveloppe du scan SQL en conservant un seul
validateur. Ne pas appliquer ce contrat à un nouveau Work en mémoire, ni
transformer toutes les requêtes de métadonnées en lectures de corps.

1. Inventorier les SELECT exacts et les sélecteurs get/list actuels, qualifier
   les contrôles et erreurs ; propriétaires mémoire/Store et identité distincts.
2. Écrire et exécuter les contre-exemples sur fixtures isolées avant correction.
3. Raccorder les lecteurs de corps concernés au noyau commun, garder les
   transactions, réservations, conflits et refus métier existants.
4. Vérifier refus avant publication/effet, absence de mutation, causes et reprise
   valide, puis frontières publiques et concurrence concernées.
5. Actualiser les contrats FR/EN, matrice d'audit, résultats et RETEX.

| Critère | Résultat observable requis |
| --- | --- |
| M29-01 | Inventaire actuel qualifié, sans confondre SELECT de métadonnées, lecteur d'identité et lecteur de corps non contrôlé. |
| M29-02 | Contre-exemples stables avec sources et échecs originaux ; aucune modification de base réelle ni suppression de trigger. |
| M29-03 | Sources de résumé, export et autres lecteurs raccordés refusent l'enveloppe invalide sans publier un paquet ou poursuivre un effet ; pas de résultat partiel utilisable. |
| M29-04 | Refus sans mutation durable ; causes conservées, cas valide et reprise vérifiés ; création à révision zéro en mémoire préservée. |
| M29-05 | Contrôles ciblés normaux/race, statique, compilation et documentation cohérents avec les mêmes entrées ; limites et suites P1 explicites. |

Les lecteurs du chantier identité ne sont pas modifiés. Un rejeu de reçu qui
ne lit pas de Work courant n'est pas assimilé au rejeu de mutation M28. Aucune
authenticité, propriété ou autorisation M20 n'est déduite de cette cohérence.
Aucun serveur, fournisseur réel ou agent natif démarré. Watchdog global de test
désactivé faute de borne contractuelle ; progression observée, délais produit
conservés. Pas de modification de politique ou de migration.

## Reprise de méthode

Deux lectures ont supposé des chemins absents (`docs/MEMORY.md` et
`internal/engine/store_test.go`). Les vrais chemins sont retrouvés par `rg` :
`docs/PROJECT-MEMORY.md`, `companion_test.go`. Les échecs de recherche ne sont
pas des constats applicatifs. Les sorties trop longues du premier lot de lecture
sont remplacées par des lectures ciblées et des résumés JSON sans manifestes
d'empreintes. Aucun contrôle tronqué ne vaut preuve.

## M29-I01 — contre-exemples et correction commune

La série `red` conserve 29 sous-cas précis en échec et deux refus déjà conformes
(identité/révision de la source de résumé). Les entrées sont stables. Les quatorze
sites de lecture de corps hors identité sont raccordés au noyau, dont la lecture
transactionnelle du lancement après le premier get. La jointure de l'inventaire
utilise le même décodage d'enveloppe. Les reçus de rejeu du graphe restent des
reçus historiques ; leur contrat ne devient pas une lecture du Work courant.

Le premier contrôle `green` échoue avant collecte : l'import JSON de
`automation_schedule.go` est devenu inutilisé. Ce défaut de notre correction
est supprimé après diagnostic, sans masquer la preuve originale. Le test UTF-8
existant garde son refus/paquet vide/octets inchangés, mais reconnaît le terme
UTF-8 partagé, au lieu d'imposer la phrase anglaise de l'ancien lecteur.

## M29-I02 — remontée des erreurs et sélection des contrôles

Les séries `red-propagation` et `red-schedule` reproduisent trois cas : faux
« aucune tâche », JSON masqué par unknown_target, et incohérence masquée par la
création de programme. Le rendu terminal et le dialogue reçoivent leur Work déjà
lu ; les deux traducteurs automation ne classent que sql.ErrNoRows en absence.
Une panne SQL et une erreur JSON conservent leurs causes. Refus sans mutation et
reprise valide vérifiés après correction.

La série `targeted` passe 29 noms mais ses alternatives de regex sans `.*` ne
sélectionnent pas les familles préfixées. Ce défaut de sélection ne vaut pas
échec applicatif et ne prouve pas ces familles. `boundaries` sélectionne leurs
noms réels : 107 PASS. Audit final obligatoire des huit nouveaux parents et des
cas de concurrence réellement présents. Les 136 noms normaux et 61 sous race
se recouvrent ; 140 noms distincts, 64 parents et 76 sous-cas. Vet et compilation
réussissent. La CLI autonome vérifie quatre effets export/refus/import, sans
fournisseur réel ou watchdog global.

## Livraison et suite de l'étape

Documentation Work/mémoire/automation FR/EN et Mermaid cohérents ; pas d'asset web
ou de contexte agent embarqué modifié, donc pas de régénération. Le README et son
schéma général n'exigent pas d'autre changement pour ce contrat de lecture.
Auto-revue du diff, pas de revue indépendante. Aucun commit, redémarrage, compte
système ou activation publique d'identité effectué par M29.

`RESULTATS-MEMOIRE-25.md` contient le RETEX et la
`evidence/m29-20261017/AUDIT-LECTURES.md` qualifie les 27 SELECT initiaux. MEM-P1
reste **en cours** : propagation des boucles/callbacks, vues complémentaires,
captures dynamiques de restauration et injection spécifique pendant la sonde de
lancement gardent Codex comme propriétaire et une preuve concrète à produire.
Le scanner AST et le passage de 168 à 167 sélecteurs ne constituent pas un audit
exhaustif de ces effets. Aucune dépendance d'identité ou conformité M20 clôturée.
