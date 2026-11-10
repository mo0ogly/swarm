# PAEA — incrément 01, revue et reprise

[English](../../../en/plans/actions-confiance/paea/INCREMENT-01.md) ·
[Plan](../PAEA-APEX.md) · [Contrat](../../../architecture/PAEA-CONTRACT.md)

Le 25 octobre 2026, Codex réalise seul le premier socle PA-00/PA-01. Cette revue
est une auto-revue de l’intégration, sans indépendance E2/E3. Le programme et son
RETEX final restent ouverts. La dernière consigne utilisateur interdit toute
nouvelle compilation pendant son autre session ; les vérifications exécutables
ci-dessous étaient déjà terminées. Aucun processus étranger n’est arrêté.

## Référence et résultat borné

La [référence conservée](evidence/20261025-contract-qekt2uwo/baseline.json) identifie
le checkout, la branche, le commit, les sources examinées et l’instance observée
`v0.1.1-dev.65`. Son archive est une sélection d’entrées d’analyse après les premiers
changements, pas un instantané atomique du dépôt avant édition. Les modifications
étrangères et les preuves historiques Go sont préservées. Les originaux DOCX/PDF
PAEA ne sont pas modifiés.

Le nouveau contrat ne dépend pas du moteur. Il vérifie les représentations et
produit une empreinte liée au contenu exact et au contexte déclaré. L’intention
métier reste stable malgré une nouvelle clé technique. Le cycle pur interdit de
transformer UNKNOWN en refus sans effet ou de recommencer un engagement. Il ne
réserve ni intention ni effet et n’authentifie aucune de ses entrées.

La commande `protection canonicalize` réemploie ce contrat via la CLI publique,
sans ouvrir de projet. Les deux domaines et FR/EN ont été exercés dans des racines
temporaires. API, UI, stockage d’autorité, mandats exécutables et services d’effet
restent absents de cette tranche. La représentation `ApprovalBinding` n’est pas
une approbation complète ; `EffectReceipt` n’est pas une attestation authentifiée.

## Vérifications conservées

| Contrôle | Résultat et portée |
|---|---|
| [Recette permanente](../../../../tools/verification/recipes/paea-contract.json), [résultat](evidence/20261025-contract-qekt2uwo/recipe-01/result.json) | PASS : 5 tests Python, 17 tests Rust de contrat, 4 tests CLI. Quatorze vecteurs sont des entrées, pas quatorze tests supplémentaires. Aucun watchdog global. |
| Intégrité de la recette | Commande `recipe.py verify` réussie sur les originaux ; l’archive conserve les entrées déclarées, argv et sorties brutes. Ce n’est pas un inventaire exhaustif des entrées transitives du compilateur. |
| [Clippy](evidence/20261025-contract-qekt2uwo/clippy-01.stderr) | `-p swarm-protection-contract --all-targets -- -D warnings`, terminé avec succès ; bibliothèque et tests de cette bibliothèque seulement. |
| [Rustdoc](evidence/20261025-contract-qekt2uwo/rustdoc-01.stderr) | `-p swarm-protection-contract --no-deps`, `RUSTDOCFLAGS=-D warnings`, terminé avec succès avant la consigne sans compilation. |
| Revue finale statique | [Inventaire lié aux octets](evidence/20261025-contract-qekt2uwo/increment-review.json), liens locaux et diff ; aucune recompilation ni recette Cargo relancée. |

Les sources moteur de SCALING-S2 changent dans l’autre session. Les quatre tests
CLI valident le chemin hors ligne, pas un candidat natif complet. Les avertissements
de compilation des travaux moteur concurrents restent dans les sorties originales.
Aucune installation ni activation PAEA n’est annoncée. Les corrections de texte
après la recette sont inventoriées séparément ; ses archives restent inchangées.

## Revue des politiques codées en dur

| Valeur ou règle | Catégorie, origine et consommateur |
|---|---|
| Schéma 1, `paea-json-v1`, préfixe d’intention, noms d’opérations et transitions | Invariants du nouveau protocole ; bibliothèque, schéma et documentation concordants. Versionner un changement ; aucune politique opérateur ni permission créée. |
| Identifiants 1–100 ASCII, SHA256 de 64 caractères, intégrité I0–I3 | Représentations réutilisées du contrat Identité et des labels existants. Le nom d’un label ne confère aucun droit. |
| Montant décimal positif, représentation i64 ; révisions i64 positives | Limites techniques des consommateurs existants. La chaîne conserve la précision navigateur ; aucun budget ou plafond financier n’est configuré ici. |
| Devise de trois lettres, OID de 40/64 caractères non nul, branche directe | Représentations des fixtures. Existence, support, authentification du dépôt et refus des références symboliques restent au service d’effet. |
| Profondeur JSON et lecture CLI bornée | Protections techniques existantes, non augmentées. La bibliothèque n’ajoute ni taille, délai, budget, retry ni plafond opérateur. |
| Racine CLI `.`, phase `delivery`, commande hors ligne seule | Compatibilité avec les valeurs par défaut du parseur ; aucune ouverture de Root. Un `--root .` explicite est équivalent au défaut ; les valeurs effectives différentes sont refusées. |
| Montants, identifiants et empreintes des vecteurs | Entrées de test explicites, jamais configuration installée ni identité attribuée. |

Aucun nouveau choix opérateur persistant n’est introduit. La chaîne
stockage → validation → Admin/API/CLI → consommateur sera obligatoire pour les
politiques de PA-02/04/05 ; elle n’est pas revendiquée par ce calcul hors ligne.

## Revue des fonctions et erreurs

L’inventaire exact est dans `increment-review.json`. Fonctions examinées : les
cinq décodeurs, les quatre validateurs de structures, `canonicalize`, ses getters
et `matches_approval`, le vérificateur `State::transition`, `oid`/`branch`, le
visiteur strict et son décodeur, l’encodage canonique récursif, la commande CLI et
les trois fonctions de l’oracle Python. Les commentaires précisent la pureté,
les entrées non authentifiées, les refus et les obligations du futur consommateur.
Les raccords de `CliError`, du dispatch et de l’aide gardent leurs contrats
existants ; le module dédié porte le détail du chemin hors ligne.

- JSON, doublon, champ inconnu ou valeur invalide : cause typée conservée,
  diagnostic public statique. Le test CLI vérifie qu’un champ privé n’est pas
  réémis, puis vérifie la reprise sur l’entrée valide dans la même racine.
- Transition invalide : résultat `Err`, état d’entrée immuable ; aucun retry ni
  succès implicite. Les deux observations de réconciliation sont distinctes.
- Lecture : réutilisation de `CommandInput`, sans ouverture de base ni jeton.
  Encodage : cause transmise dans `CliError::Protection`. UTF-8 interne : cause IO
  conservée. Aucun de ces cas ne peut avoir engagé un effet projet ici.
- Écriture/flush : propagation de `CliError::Output`, sortie non nulle. `/dev/full`
  vérifie l’échec du parcours public sans création de `.swarm`. Une sortie
  partiellement écrite reste possible ; elle n’est pas une preuve d’effet.
- Publication des vecteurs : l’oracle réemploie le publieur exclusif atomique
  existant ; il ne remplace pas un ancien fichier ou une preuve.

Les tests ne prouvent pas toutes les défaillances IO ni une fermeture de service
distribué. Les mêmes contrats devront être suivis à travers le futur stockage,
transport et service d’effet ; l’absence d’effet ne sera alors plus déduite d’Err.

## État des 17 invariants

« Test local de données » signifie uniquement ce qui est décrit dans cette table.
Aucun invariant global n’est déclaré satisfait. Les chemins ci-dessous sont les
points de réemploi examinés ou à raccorder, pas des producteurs protégés attestés.

| Invariant | Acquis de cet incrément / piste de réemploi | Stockage et consommateur protégés restant à vérifier | État / propriétaire |
|---|---|---|---|
| I01 | Décodeur strict refuse les champs d’autorité inventés ; sortie CLI explicitement non autorisante. | Producteur d’admission et consommateur réel, même après injection. | Test local de données ; Identité/intégration. |
| I02 | Forme principal/projet/instance/mandat ; réemploi du contrat Identité. | Autorité IA3, délégation, audience, liaison au projet ouvert. | Non démontré en exécution ; Identité. |
| I03 | Sources, versions et labels validés ; ordre des sources conservé. | `memory_files.rs`, lectures/résumés/transmissions et destinataires réels. | Test local de données ; Mémoire/AC-02. |
| I04 | Empreinte et comparaison d’accord changent avec contenu ou politique. | Contenu réellement consommé dans grand livre/Git et approbateur habilité. | Test local de données ; Trust/dépôt/UX. |
| I05 | Contexte inclus dans l’empreinte ; aucune vérification de fraîcheur. | Mandats, politique et révocation courants au commit. | Spécifié ; Identité/services d’effet. |
| I06 | Aucun mécanisme de confinement ajouté. | `agent_launch.rs` et tous descendants, fichiers, IPC et réseau ; inventaire fermé. | Non démontré ; AC-02/lancement. |
| I07 | CLI ne lit aucun secret ni jeton. | Clés hors domaine agent, séparation de comptes et usages de secrets bornés. | Non examiné à la frontière OS ; Identité/AC-02. |
| I08 | Clé métier dérivée, stable malgré nouvelle clé ou contenu alternatif. | Réservation durable dans grand livre/Git, conflits et cumul. | Test local de données ; services d’effet. |
| I09 | Cycle UNKNOWN limité à réconciliation ; états terminaux non mutants. | Intention persistée, crash, réponse perdue et observation indépendante. | Test local de données ; services d’effet/vérification. |
| I10 | Forme d’approbateur uniquement. | Authentification, habilitation, séparation des tâches et étude de vigilance. | Non démontré ; UX/commanditaire/évaluateur. |
| I11 | Forme du reçu distingue observations et autorisation. | `trust/proof.rs` et `root_store/trust_store.rs` à réemployer avec observateur et ancres séparés. | Authenticité/indépendance non démontrées ; vérification. |
| I12 | Entrées invalides et opérations actives non implémentées refusent sans ouvrir Root. | Indisponibilité des contrôles au lancement et au service d’effet. | Test local limité ; AC-02/services d’effet. |
| I13 | Labels représentés, sans contrôle de flux. | Courtier et sorties réellement reçues, modèles et transports compris. | Non examiné en exécution ; AC-02/Mémoire. |
| I14 | Aucun canal A2A ajouté. | Authentification des pairs, audience/mandat/provenance au destinataire. | Non démontré ; Identité/Mémoire. |
| I15 | Empreinte du manifeste liée à l’action, sans mesure du binaire. | Catalogue effectif, outil consommé et réadmission après substitution. | Forme implémentée ; catalogue/lancement. |
| I16 | Révisions positives, aucun temps fourni par agent consommé. | Epochs, baux monotones, ordre révocation/engagement et restart. | Non démontré ; Identité/services d’effet. |
| I17 | Erratum sur le jeton à usage unique conservé. | Jeton préémis refusé après révocation avant première consommation. | Spécifié, preuve manquante ; service d’effet/évaluateur. |

Le stockage Trust et les références Git existants ne sont pas extraits ni migrés
par cette tranche. Les preuves Go historiques et les tests Rust antérieurs gardent
leur portée propre ; aucun verdict n’est transmis automatiquement à PAEA.

## Découvertes, moyens et reprise

| Origine / valeur | Décision et état | Propriétaire et critère de reprise |
|---|---|---|
| PAEA §6.2 : unicité du jeton ≠ révocation avant consommation. | Erratum PAEA-ERR-01 spécifié, original conservé. | Service d’effet ; témoin valide puis révocation et première consommation sans effet. |
| Reprise de la session : autre chantier modifie les entrées moteur du même checkout. | Préserver les changements ; pas de livraison d’un candidat cumulatif non revu. | Intégrateur local ; chantier disponible, revue et qualification du candidat complet. |
| Consigne explicite de l’utilisateur : éviter de compiler. | Aucun nouveau build/test Cargo, aucun redémarrage. Documentation et revue statique seules après la consigne. | Utilisateur/intégrateur ; consigne levée avant toute compilation. |
| Réponse utilisateur : pas d’évaluateur ni d’équipe tierce. | Protocole PA-11 en brouillon, campagnes PA-12 non lancées. | Commanditaire ; moyens et acceptation des responsabilités d’indépendance avant préenregistrement. |
| Une empreinte ou un schéma peut être pris pour une preuve d’autorité. | Sortie `not-established` et contrats explicites ; limite vérifiée sur CLI hors ligne. | Producteurs futurs ; aucune API active ne doit accepter le contexte fourni comme autorité. |
| Relecture des options CLI : `--root .` reste accepté comme défaut. | Texte FR/EN corrigé pour décrire la valeur effective réellement refusée. | Codex ; revue statique du prédicat et documentation concordante. |

Les comptes séparés, privilèges nécessaires, canal authentifié et restrictions
Linux effectives restent à qualifier avec IA3/AC-02. Aucun outil installé ou noyau
récent n’est pris comme preuve. Les fournisseurs scriptés sont les seuls moyens
requis du socle ; comptes, modèles et budgets de campagnes réelles ne sont pas
inventés. Les apprentissages sont transmis par les plans FR/EN et la file APEX,
pas par une modification des contextes embarqués ou d’un processus déjà démarré.

La prochaine réalisation est la finalisation PA-01 avec les producteurs IA3,
puis les deux services PA-03 après PA-02. Elle reste soumise aux dépendances de
migration et de confinement conservées dans le plan. Le programme n’est pas clos.
