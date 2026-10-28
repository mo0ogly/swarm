# M26 — reçu extractif contrôlé avant le fournisseur

17 octobre 2026, Codex seul, reprise replan/APEX, revue locale code-reviewer et
vérification du parcours verify-fix. Pas de revue indépendante ni d'acceptation
globale M20. Le serveur utilisateur et les fichiers identité sont préservés.

## Résultat dans le parcours existant

Le premier lancement extractif M19 vérifie désormais la cohérence de la demande
confirmée, du reçu et du contexte juste avant le sous-processus fournisseur.
Le contrôle retrouve `agents.request` avant de décider qu'il n'y a pas de
sélection : supprimer `context.summary` ne désactive plus la garde ; ajouter un
reçu à un lancement sans sélection est refusé.

`memorySummaryExtract` partage la construction des octets entre admission et
contrôle. La sélection et son digest, les listes ordonnées reçues/omises, le
compte de sources, les empreintes/tailles, les identifiants et révisions doivent
correspondre au reçu reconstruit. Le préfixe préparé doit correspondre à la
confirmation originale et se terminer par les extraits attendus. Le manifeste
courant couvre le prompt complet, y compris une reprise moteur ajoutée ensuite.

Le contexte préparé ne peut pas prétendre avoir déjà reçu un contrôle au départ.
Le reçu courant peut être observé de nouveau sans réécrire cette histoire. La
copie appelante n'est actualisée qu'après la sauvegarde réussie ; si cette
sauvegarde échoue, le fournisseur ne part pas. Le superviseur peut alors
conserver un état `failed` et la cause originale sans fausse estampille.
Aucun nouveau champ public, schéma de stockage ou réglage opérateur n'est ajouté.

## Défauts conservés et corrections

**M26-I01 — reçu déconnecté de la confirmation.** Les deux variantes rouges
`edited-selection` et `missing-receipt` démarrent réellement le fournisseur
scripté avant correction et terminent `completed`. Trois noms FAIL (parent et
deux sous-cas), plus le paquet ; zéro PASS, entrées stables. Le source exact et
les sorties sont conservés dans `red-*`. L'altération est injectée dans une
fixture isolée : aucun accès pirate via API ou navigateur n'est prétendu.

**M26-I02 — échec d'écriture et test du mauvais observable.** L'ancienne garde
modifiait l'appelant avant son écriture ; le traitement d'échec répétait alors
l'écriture estampillée et échouait sur le même trigger. La copie préservée
permet désormais de sauver normalement l'échec. L'ancien test attendait une
erreur technique du superviseur : il vérifie maintenant l'état `failed`, la
cause, l'absence de marque de contrôle et d'appel fournisseur, puis la
persistance après réouverture. Aucun trigger de protection n'est retiré.

**M26-I03 — fixture périmée face à une garde plus en amont.** Le trigger de
projection identité refuse l'injection de Work UTF-8 malformé avant la CLI
mémoire. Le test vérifie ce refus sans mutation. Le garde UTF-8 du lecteur est
aussi exercé sur une fixture SQL minimale distincte, sans mission migrée : aucun
trigger réel n'est supprimé pour atteindre ce cas. La première série conserve
167 noms PASS et trois noms FAIL, plus le paquet, avec entrées stables.
Diagnostic : `candidate-diagnosis.json` ; sources initiales dans `before/`.

**M26-I04 — fausse observation dans le contexte préparé.** La revue locale
constate que la première comparaison efface les champs d'observation avant de
comparer l'histoire préparée. Le nouveau test échoue réellement sur ce défaut
avant correction (`review-red-*`, un nom FAIL et le paquet). La garde refuse
maintenant ces champs dans `prepared_context` et conserve le réexamen du seul
reçu courant. Les deux versions et leurs preuves restent distinctes.

Deux lectures de chemins supposés et une lecture trop précoce de reçu ont aussi
échoué pendant la méthode ; aucun résultat n'en a été inféré. La reprise cherche
les symboles dans les chemins réels et attend la fin du processus avant de
consommer son reçu. Détails conservés dans le plan, sans effacer ces erreurs.

## Vérification liée aux sources

État initial : `evidence/m26-20261017/baseline.json`, `status-before.txt`,
préimages sous `before/`. Les lanceurs enregistrent commandes, dates, sorties
JSON originales, noms exécutés, toolchain et empreintes des dépendances locales
Go réellement compilées (dont les ressources embarquées). `go test -timeout=0` ;
aucun timeout Python. Les délais fonctionnels du produit sont conservés ; le
travail du compilateur est observé, sans borne globale héritée.

La série verte initiale `final` compte **171 noms PASS : 36 tests principaux et
135 sous-cas**, zéro FAIL/SKIP ; statique et compilation réussies. Après le
contre-exemple M26-I04, la série `verified` revérifie les parcours affectés : **81
noms PASS, 21 tests principaux et 60 sous-cas**, zéro FAIL/SKIP ; statique et
compilation finales réussies, entrées stables.

Les 92 gardes source/usages M24/M25 restantes sont réutilisées : entre ces deux
séries, seuls `memory_summary_receipt.go` et son test changent ; ni le constructeur
partagé ni leurs producteurs/appelants ne changent. Elles ne consomment pas le
nouveau garde d'historique et aucun init n'est ajouté. Cela représente **173
noms distincts couverts**, avec la réutilisation explicitement liée au diff et
aux entrées dans `audit-final.json`, pas une nouvelle exécution unique des 173.

Le premier lot race conserve 31 noms PASS sur son candidat. Le lot final
`verified-race` donne **33 noms PASS : sept tests principaux et 26 sous-cas**,
zéro FAIL/SKIP, entrées stables. Il cible la garde complète, refus, panne/reprise, observations historiques et
courantes, entrée fournisseur et confirmation concurrente. Ce contrôle n'est
pas une preuve d'engagement atomique entre lecture et départ.

| Critère | État / preuve locale |
| --- | --- |
| M26-01 | PASS : fournisseur scripté effectivement appelé dans les deux variantes rouges ; refus avant appel après correction. |
| M26-02 | PASS : 24 variantes de reçu/contexte incohérent et huit de demande invalide refusées sans résultat favorable ni mutation ; retour au cas valide vérifié. |
| M26-03 | PASS : contexte confirmé, extraits, reprise ajoutée et prompt complet concordent ; changement du préfixe, digest/tailles falsifiés refusés. |
| M26-04 | PASS : erreur SQL conservée, appelant inchangé, reprise après retrait du défaut injecté ; départ fournisseur empêché et état `failed` relu après réouverture. Historique préparé conservé, observations courantes renouvelables. |
| M26-05 | PASS : vrais Store, CLI/API en processus, supervision et sous-processus scripté ; octets reçus comparés au prompt/reçu, sources historiques conservées, refus d'authentification/API et idempotence couverts. Aucun compte fournisseur réel. |
| M26-06 | PASS local : statique/compilation, race final, formatage/diff, liens et empreintes dans `audit-final.json`. Revue du même auteur ; aucune acceptation indépendante. |

Les sorties sont dans `red-*`, `candidate-*`, `final-*`, `race-*`, `review-red-*`,
`verified-*`. Le diff possédé et l'audit final lient les fichiers au résultat.
Les variantes invalides directes ne modifient que leur fixture ; aucune base de
mission réelle n'est employée. Les tests CLI/API sont en processus avec le code
compilé par Go, pas une recette navigateur ou un test du binaire déjà installé.
Aucun écran/asset n'est changé ; pas de nouvelle recette navigateur. Les libellés
du schéma séquence FR/EN sont alignés, rendu graphique non vérifié dans ce lot.

## Limites, reprise et RETEX

Ce contrôle est une concordance locale avec la demande conservée, pas une
attestation face à un acteur capable de réécrire conjointement demande, données
et preuves. Il ne crée ni propriétaire, auteur, identité ou permission ; il ne
réserve pas les sources jusqu'à l'effet et ne supprime pas la fenêtre après
l'observation. Les dix conformités publiques M20 restent ouvertes.

Le chantier identité a adopté le pilote local sous compte système distinct et
prévoit une autorité distante ultérieure ; ce choix n'est plus à redemander.
Le principal humain initial, le service protégé effectivement vérifié et le
raccord courant restent les préconditions de l'activation. Propriétaires :
identité pour le contrôle protégé/avant-plan et le service ; mémoire pour les
attributions persistées et consommateurs ; opérateur pour le principal initial.
Aucun message inter-chat, compte/service, fournisseur réel, mission native,
redémarrage, commit, publication ou déploiement n'est réalisé ici.

La reprise de fixture après suppression du défaut injecté démontre le garde,
pas un outil public de réparation d'une base corrompue. Les échecs historiques
restent conservés. La sélection M19 reste réservée au premier lancement manuel
autorisé ; aucune reprise extractive ou hiérarchique supplémentaire n'est créée.

Leçon réutilisable : vérifier la demande d'origine et la cohérence complète au
consommateur, pas seulement la fraîcheur d'une source ; préserver l'appelant
jusqu'au succès d'écriture et distinguer échec pris en charge et succès métier.
Une garde nouvelle peut invalider une fixture de corruption plus ancienne :
tester la première frontière réelle et isoler explicitement le lecteur sans
retirer la garde. Transmission par les tests nommés, le constructeur partagé
et les docs FR/EN du consommateur. Les consignes communes couvrent déjà cette
méthode ; aucun contexte embarqué ni processus lancé n'est présenté comme mis à
jour par cette documentation. Le RETEX du lot ne clôt pas le RETEX global M20.
