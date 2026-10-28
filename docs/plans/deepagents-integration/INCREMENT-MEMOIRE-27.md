# M31 — MEM-P1, erreurs du conducteur

17 octobre 2026, reprise « on passe a la suite ». Codex seul, replan/audit-pdca.
Préimages et reçus dans `evidence/m31-20261017`. Préserver les travaux M29/M30
et ceux de la session identité ; aucun déploiement, modèle réel ou commit.

## Observation et décision

`missionOwnedCycle` abandonne sur erreur de liste ou d’occupation sans mettre
à jour les baux connus. `checkMissionSupervision` renseigne une réconciliation
même en échec et peut réactiver un bail arrêté. Le watch borné à une mission
lit pourtant toutes les missions. Hypothèses à reproduire avant correction.

Retenir la propagation des causes, le diagnostic existant et des transitions
erreur/reprise atomiques dans le journal de coordination existant. Un échec
global ne revendique aucune mission illisible : seuls les baux déjà possédés
et non arrêtés peuvent recevoir ce diagnostic. Le contrôle stockage reste
prioritaire et interdit les écritures. Aucun nouveau plafond ou réglage.

## Critères et séquence

| Critère | Preuve attendue |
| --- | --- |
| M31-01 | Contre-exemples stables avant correction, bases isolées, sans suppression de protection. |
| M31-02 | Erreur d’inventaire/occupation remontée, diagnostic des baux possédés ; aucune mutation métier ou consommation. |
| M31-03 | Première erreur sans fausse réconciliation ; bail arrêté/remplacé jamais réactivé ; transitions atomiques sans spam de polling. |
| M31-04 | Watch ciblé isolé d’un Work étranger invalide ; erreur/reprise observées en CLI et état partagé HTTP/CLI. |
| M31-05 | Vérifications ciblées/race, vet/build, sources stables, docs FR/EN, auto-revue et RETEX avec limites. |

Boucle : reproduire → corriger → vérifier refus/reprise/concurrence → documenter
et réexaminer MEM-P1. Go `-timeout=0`, subprocess sans watchdog global faute de
borne contractuelle ; progression suivie. Les politiques produit sont inchangées.

Le lecteur/callback historique sans bail doit aussi transmettre la cause. Les
callbacks de fraîcheur booléens, vues de repli et enveloppes Agent restent une
suite qualifiée de P1 ; P2–P5 et les droits M20 ne sont pas livrés par ce lot.

## M31-I01 — reproduction et compatibilité

La première série rouge conserve six scénarios en échec : inventaire silencieux,
premier contrôle en échec présenté comme réconcilié, bail arrêté réactivé, watch
ciblé bloqué par une enveloppe étrangère, callback privé de cause et contrôle
validé sans trace durable. Le sous-cas de bail remplacé passe déjà. Entrées
stables, reçus `red-*` conservés. La première correction passe ces cas.

Le lot élargi (`candidate`) révèle un refus correct dont le nouveau libellé ne
respecte plus le préfixe historique attendu par le coordinateur. Restaurer ce
préfixe sans changer le refus ni le test existant. La revue constate aussi que
le texte vide ou réduit à des contrôles peut transformer une erreur non nulle
en succès : deux contre-exemples `red-empty` reproduisent ce défaut avant
normalisation descriptive. L’erreur retournée conserve sa cause originale.

La lecture d’un script CLI supposé inexistant est également conservée comme
récidive de méthode : le fichier réel est localisé par `rg --files` avant reprise,
sans relance du chemin inventé. Les preuves ne dépendent pas de cette lecture.

L’arrêt des baux et la trace idempotente d’acquisition ne doivent plus absorber
leurs erreurs : retourner/propager l’arrêt, et retenter la trace d’acquisition
avant tout effet métier. Aucun bail d’un autre conducteur n’est revendiqué.

## Livraison et reprise

M31-01 à M31-05 vérifiés localement sur `final` : 112 noms normaux (80 parents,
32 sous-cas), 23 sous race inclus (17 parents, six sous-cas), vet/build, quatre
assertions de la vraie CLI/watch et HTTP authentifié. Reçus stables et sources
rouges exactes conservés ; deux Mermaid FR/EN rendus et inspectés. Auto-revue,
pas d’acceptation indépendante, serveur existant inchangé. Documentation et
RETEX : `RESULTATS-MEMOIRE-27.md`. Suites P1 attribuées dans `AUDIT-REPRISE.md`.
Les 37 fichiers identité observés et les entrées Go hors M31 restent intacts.

## M31-I02 — version courante et assets concurrents

Le contrôle des empreintes après `final`/`race` refuse huit assets web embarqués
modifiés depuis les séries stables. Les trois sources Go M31 et les 37 fichiers
identité observés n’ont pas changé. Conserver l’assertion et son diagnostic dans
verification-concurrent-assets.json ; aucun asset concurrent écrasé. Les reçus
antérieurs restent valides pour leurs octets, pas pour une compilation courante.
Requalifier les tests backend, race et le vrai binaire avec les assets courants,
puis rejouer la CLI et le contrôle exact des entrées. Cela ne fournit aucune
acceptation visuelle des modifications concurrentes. Les recettes initiales sont
archivées avant leur paramétrage, leurs empreintes historiques restent vérifiables.

Requalification achevée : `refresh` 112 noms, `race-refresh` 23 noms inclus,
vet/build et `cli-refresh` quatre assertions PASS ; entrées stables. Les huit
assets concurrents sont préservés. Aucun critère de UI concurrente accepté ici.
Le contrôle final compare empreintes et inventaire actuel des entrées compilées.
