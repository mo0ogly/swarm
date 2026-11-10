# IA3c.2c — document d’autorité strict, données sans capacité implicite

17 octobre 2026 — **PASS du contrat local et de la compatibilité ciblée moteur.**
L’enrôlement réel, l’admission publique et la fraîcheur à l’usage restent ouverts.

`internal/enrollmentauthority` ajoute le schéma humain versionné, un producteur
qui vérifie avant signature et un consommateur d’énoncé authentifié qui compare
tout son contexte attendu. Le journal dérive la génération de l’événement de
création vérifié. `internal/identitycontract` partage les règles avec le moteur,
sans créer de rôle ou de permis à partir de JSON.

## Besoin, décision et périmètre

Le corps opaque d’IA3c.2b ne devait pas devenir une autorité par sa seule signature.
Deux options : recopier les validations moteur ou extraire un contrat pur partagé.
L’extraction est retenue pour éviter deux définitions du principal, de la portée
et des permissions. Les types moteur restent privés ; leurs champs JSON et règles
sont conservés. `safeName` délègue la même syntaxe ASCII de 1 à 100 octets : limite
de représentation existante, sans ajout de politique opérateur.

Le [plan](../../identites-agents-replan.md) conserve la décision avant code et les
reprises. Le [contrat cible](../../identites-agents-enrolement-IA3c2.md) et les
contrats opérationnels [FR](../../../../architecture/AGENT-IDENTITY-CONTRACT.md)
/ [EN](../../../../en/AGENT-IDENTITY-CONTRACT.md) décrivent les onze champs,
l’encodage canonique, les contextes attendus, erreurs et limites.

Travail direct Codex seul, aucune délégation, modification de mission, clé réelle,
compte système, service, fournisseur externe, activation, commit ou déploiement.
Le nouveau paquet n’est importé par aucun endpoint ou lanceur. Le paquet commun
est utilisé par le moteur ; cela ne raccorde pas la source d’enrôlement au moteur.

## Preuves et critères

| Critère | Résultat observé | Preuve / limite |
| --- | --- | --- |
| Contrat commun | 9 noms PASS | Syntaxe existante, bornes 100/101, principal opaque distinct d’une ressource glob |
| Autorité typée | 63 noms PASS | Schéma strict, refus, contexte, dates, génération, réouverture |
| Journal modifié | 42 noms PASS | Lot antérieur rejoué car son cadre Record a changé |
| Ensemble normal | 114 noms PASS, 0 FAIL, 0 SKIP | [normal-04](normal-04-run.json), trois paquets effectivement collectés |
| Même ensemble race | Mêmes 114 noms, 0 FAIL/SKIP | [race-04](race-04-run.json), aucune course signalée |
| Analyse/compilation des paquets | PASS | [vet](vet-04-run.json), [build](build-04-run.json), 13 entrées inchangées |
| Compatibilité moteur ciblée | 173 noms PASS, 0 FAIL/SKIP | [engine-normal-03](engine-normal-03-run.json), contrat, chaînes, émetteur et reçus |
| Analyse moteur | PASS | [engine-vet-03](engine-vet-03-run.json) |
| Compilation application | PASS | [application-build-03](application-build-03-run.json), binaire de fixture temporaire, non déployé |
| Entrées moteur/application | 1 069 inchangées pendant chaque contrôle 03 | Go/tests, ressources embarquées et testmain généré, [inventaire](engine-normal-03-inputs.json) |
| Documentation | FR/EN et schémas actualisés | [4 rendus](diagrams.json), [2 PNG inspectés](visual-inspection.json) |
| Conservation | 12 fichiers précédents inchangés, 3 modifiés comme prévu | [préservation](preservation.json), diffs par rapport à la reprise |

Les comptes incluent parents et sous-cas ; ils ne désignent pas autant d’attaques
indépendantes. Les commandes, heures, retours, JSONL/stderr et empreintes sont
conservés. Les tests ont tous `-timeout=0`, sans watchdog global arbitraire.
Le moteur n’a pas de nouveau test race ici : son changement est une extraction
pure, sans synchronisation modifiée. Le nouveau schéma et le journal sont testés
avec race. Aucun lot global `go test ./...` ni recette navigateur n’est revendiqué.

Le contrôle initial [schema-red-01](schema-red-01-run.json) échoue à compiler car
les nouvelles interfaces n’existent pas : zéro test exécuté, aucune preuve de
vulnérabilité du produit. Les lots 01/02 sont intermédiaires. Les sources finales
et archives locales sont liées par [source-final](source-final.json). L’inventaire
moteur référence aussi des fichiers partagés non archivés intégralement ici ; sa
reproduction exige leurs versions exactes, pas leurs seules empreintes.

## Propriétés vérifiées et limites

Le document impose un principal `human` complet et exact : ID local, kind, issuer,
sujet. Ni identité locale/service, ni homonyme d’un autre issuer/sujet ne passe.
Le projet, la portée/version/digest du plan, la révision Trust et la demande sont
comparés exactement à des attendus indépendants. Les permissions sont non vides,
uniques et exactes ; les délégables sont un sous-ensemble valide sans doublon.
Les fenêtres sont explicites, inclusives au début et exclusives à l’expiration.
UTC est un encodage canonique, aucune durée n’est calculée par défaut.

`Encode`/`Decode` valident structure et encodage ; ils ne prouvent pas la validité
courante. `DecodeActive` applique l’instant d’observation fourni, mais ce paramètre
n’est pas une preuve d’horloge fraîche. Clés inconnues, doublons imbriqués,
variantes de casse, omissions/null, types erronés et encodages différents sont
refusés. Aucun document partiel n’est retourné sur erreur. Un JSON structurellement
valide reste une donnée et ne permet pas de reconstruire une capacité privée.

La génération doit égaler le prochain événement chez `SignEnrollment`, puis
`Record.EnrolledSequence` chez le consommateur. Le journal calcule cette séquence
en rejouant sa chaîne vérifiée ; le corps ne peut pas la déclarer à sa place.
Le test ajoute d’autres événements, vérifie la stabilité de la génération, et
emploie volontairement le signataire générique pour écrire un corps avec une
fausse génération : le consommateur le refuse. La propriété reste vérifiée après
réouverture. Les événements signés et le schéma SQLite sont inchangés ; les cadres
ActiveReply ajoutent `enrolled_sequence`. Les anciens cadres sans ce champ ne
satisfont pas le nouveau consommateur strict ; aucun client moteur n’en dépendait.

Un Statement ancien peut toujours passer avec son ancien Expected : le test
conserve ce cas puis prouve son refus contre une nouvelle tête après révocation.
Cela vérifie la liaison au témoin, pas la fraîcheur de ce témoin. Une signature,
un nonce, une génération exacte ou un document valide ne remplacent ni source
courante protégée, ni point de linéarisation avec publication/création/départ.
Pas de droits mémoire appliqués, descendance arrêtée, compte humain externe,
MFA, approbation indépendante, isolation OS ou disponibilité distribuée prouvés.

## Replan de la méthode et du dépôt partagé

1. Le lanceur 02 hashait les nouveaux tests communs sans les inclure dans la
   commande : remplacement textuel erroné (`-count` au lieu de `-count=1`). Les
   104 noms de ce lot n’incluent donc pas ces neuf cas. Correction des commandes
   et garde de collecte par paquet ; les lots 03/04 les exécutent effectivement.
2. La première collecte moteur échoue avant tests : un testmain généré est dans
   le cache Go, hors dépôt. Script original conservé ; l’erreur initiale est
   référencée au résultat d’outil, sans prétendre disposer d’un stderr fichier
   original. Le correctif conserve aussi cette entrée et son empreinte absolue.
   Aucun test n’est compté pour cette précondition en échec.
3. Les contrôles moteur 02 passent sur 1 068 entrées stables. Le chantier mémoire
   modifie ensuite trois fichiers et ajoute une entrée au graphe courant. Les
   originaux 02 restent historiques ; aucune modification de ces fichiers par
   ce chantier identité. Les contrôles 03 vérifient le nouvel état à 1 069 entrées,
   toutes stables pendant chaque contrôle. [Révisions](engine-input-revisions.json).
   Des modifications partagées ultérieures nécessitent leurs propres preuves.

Ces reprises corrigent des préconditions concrètes ; ni attente augmentée,
reformulation ou simple relance inchangée. Les empreintes des binaires 02 et 03
sont conservées séparément ; ils ont été construits en répertoires temporaires
puis retirés, sans toucher au service en cours. Aucun « build vert » ne qualifie
la version actuellement servie ou l’acceptation globale.

## Améliorations et suite autorisée

| ID / origine | Valeur et périmètre | Propriétaire / état / critère |
| --- | --- | --- |
| I2c-01, lecture des deux contrats | Une source de validation évite la divergence producteur/consommateur | Codex ; réalisé, règles communes et compatibilité ciblée vérifiées |
| I2c-02, cadrage de génération | Distinguer numéro signé et événement de création authentifié | Codex ; réalisé et vérifié après autres événements et réouverture |
| I2c-03, collecte de tests | Une empreinte de test ne prouve pas son exécution | Codex ; garde par paquet réalisée, neuf noms communs réellement exécutés |
| I2c-04, testmain externe | Inventaire fidèle des entrées compilateur | Codex ; correction vérifiée sur 1 069 entrées, sources cache incluses |
| I2c-05, entrée partagée modifiée | Distinguer résultat historique et candidat courant | Codex + chantier mémoire ; preuves 02 préservées, nouvel état vérifié en 03 |
| I2c-06, futur client externe | Versionner explicitement le cadre de lecture avant son adoption externe | Codex ; conservé, différé car aucun client déployé ; critère : version négociée et refus des cadres inconnus avant premier raccord externe |

Le prochain contrat commun restant est EP-02 : sérialiser et couvrir la demande
complète résolue (principal/session vérifiés, projet/plan, événement, instantané,
racines et checkpoint), puis préparer la conversion autorisée du résultat en
capacité moteur. Le service réel requiert toujours choix opérateur d’hébergement,
principal humain initial, racine épinglée et témoin indépendant durable. Ce travail
ne peut pas déduire ces choix d’un « next ». F01–F07, IDA, IA3 public, IA4/IA5 et
RETEX global restent ouverts. Pas de revue indépendante ni d’acceptation globale.
