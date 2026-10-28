# M21 — vérifier la liaison des citations à leur échange source

Reprise du 16 octobre 2026 : l'utilisateur a ouvert la conversation dédiée aux
identités. Codex seul ; `replan`, puis correction bornée avec APEX.

## État réel et séparation des responsabilités

La session « l'audit des identités » a livré IA1 : sept contre-exemples, aucune
correction applicative. Son rapport et son plan ont été consultés en lecture
seule. Aucun message inter-chat, agent délégué, appel de modèle ou lancement
Swarm natif n'est autorisé ou nécessaire pour M21.

Le chantier identité conserve IA2/IA3 et les sources communes de lancement,
stockage et Trust. Cette reprise possède les outils de préparation mémoire,
leurs tests Python, `docs/PROJECT-MEMORY.md`, son équivalent anglais et le suivi
`deepagents-integration`. Les 13 contrats Go lus sont empreintés et restent
inchangés. M20, attribution et droits effectifs, reste ouvert jusqu'à un contrat
d'identité/d'autorisation réellement opérationnel.

## Découverte et décision

Origine : vérification du contrat entre `memorySummarySourceForWork` et
`swarm_source.prepare`. Le producteur Go dérive chaque identifiant de citation
de la référence JSON `task`, `attempt`, `sha256`, avec le suffixe question/réponse.
Le convertisseur vérifie contenu, rôles et empreintes, mais ne contrôle pas cette
liaison. Il accepte des identifiants renommés et un échange dupliqué sous des
identifiants distincts. Le préparateur `admission.selection` consomme le même
validateur et accepte aussi une référence extractive reconstruite sur ces alias.

Preuve avant correction : douze tests du convertisseur, dont trois méthodes en
échec pour cinq variantes ; sept tests du choix, dont deux nouvelles méthodes en
échec. Les originaux sont dans `evidence/m21-20261016/before-*`. Ce sont des
défauts de cohérence de préparation hors moteur, pas une preuve d'usurpation
d'agent réussie ni de contournement de l'admission Go.

Deux options : ajouter des contrôles indépendants à chaque CLI, ou corriger le
validateur partagé depuis le contrat Go. La seconde est retenue : même refus
pour la préparation et le choix, sans changer le schéma d'export ni les droits.
Les fixtures Python doivent utiliser les identifiants réellement produits par
Go ; leurs anciens alias `q` / `a` masquaient le défaut.

## Réalisation et critères

| Lot | Propriétaire / dépendance | Effet observable / vérification |
| --- | --- | --- |
| M21.1 | Codex ; M19 conservé | Reproduire le renommage, le doublon et leur propagation au choix, avant toute correction. Réalisé, échecs conservés. |
| M21.2 | Codex ; M21.1 | Recalculer la liaison depuis les champs de la référence, avec la sérialisation Go ; refuser alias et répétition avant toute écriture. Compatibilité avec ordre JSON différent et ancien `attempt` vide. |
| M21.3 | Codex ; M21.2 | Tests Python ciblés, mêmes sources préparées via les deux CLI et parcours source Go → référence → choix. Aucun modèle réel ; données et reçus historiques conservés. |
| M21.4 | Codex ; M21.3 | Documentation FR/EN, recette, rapport et RETEX ; vérifier les empreintes des fichiers communs et distinguer les preuves sur binaire figé d'un déploiement. |

Résultat de réalisation : M21.1–M21.4 réalisés et vérifiés dans leur périmètre
local ; [rapport et RETEX](RESULTATS-MEMOIRE-17.md). Les 57 tests Python passent,
les quatre vecteurs concordent et la recette publique passe cinq contrôles M19
et cinq M18 sur le binaire M19 conservé, avec trois invocations scriptées et
zéro modèle réel. L'inventaire M20 consolide les treize consommateurs ; leurs
droits effectifs restent ouverts, ainsi que les identités IA2/IA3.

Aucun champ propriétaire/auteur/principal n'est fabriqué. Une empreinte correcte
établit une liaison de contenu ; elle n'établit ni authenticité, ni droit d'usage,
ni autorisation de lancement. La protection demandée en M20 dépend toujours
d'IA2/IA3. Les nouvelles découvertes utiles rejoignent ce suivi avec leur preuve,
propriétaire et condition de reprise.
