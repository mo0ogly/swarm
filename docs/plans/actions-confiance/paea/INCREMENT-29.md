# PAEA29 — bootstrap consommé une fois et connaissance courante

## Replan / contrat avant réalisation

28a cinq tests du secret personnel et neuf du noyau passent ; CLI/build restent
ouverts pendant le travail étranger de publication cumulative. Poursuivre le
travail indépendant : le futur service doit consommer la preuve initiale une
seule fois et retenir l'autorité du compte pendant l'usage, pas seulement vérifier
un mot de passe puis laisser une révocation gagner avant l'effet.

Réutiliser le vrai journal signé, connexion retenue, checkpoint indépendant et
révocation irréversible. Un journal personnel distinct sépare son stockage du
journal de droits métier : ordre de réservation compte → droits → configuration,
jamais l'inverse. Pas de seconde base/mécanisme de hash/timer ad hoc. Le journal
actuel applique un événement par transaction : ajouter l'application atomique d'un
lot signé général, puis consommateur personnel. Aucun cas spécial mission/date.

29a : lot signé entièrement vérifié, CAS de base/chaîne, tout ou rien et reprise
exacte sans doublon, refus préservé, vraie contention ; régression single apply.
29b : bootstrap canonique épinglé avec engagement SHA256 d'un secret aléatoire
32octets remis hors canal agent, principal/installation/génération/coûts explicites.
Préparation après preuve correcte ; révocation bootstrap et création du record
personnel dans la même transaction. Proposition exacte conservable pour reprise
incertaine ; pas de recréation après consommation/révocation.
29c : mot de passe vérifié sur record signé actif au checkpoint indépendant,
réservation retenue pendant callback ; aucun autre champ d'acteur. Refus avant
callback, autorité perdue après callback conserve son résultat et l'incertitude.
29d : recette permanente, sources/revue/docs FR/EN ; raccord CLI/build28 repris
uniquement sur précondition réellement changée. Pas de compte humain réel,
service/confinement ou consentement déduit des fixtures. La provenance du secret
initial, les checkpoints indépendants et l'intégration transport/Admin/UX restent
prérequis de l'activation. Codex propriétaire technique, opérateur enrôlement réel.


## Résultats actuels et reprise de livraison

`recipe-local-03` : **35tests nommés /6étapes PASS**, intégrité VERIFIED ;
5dérivé,4lot atomique,5compte personnel,9journal historique,3connexion retenue,
9noyau de configuration. Rustdoc état/autorité passe avec liens cassés refusés.
Le premier lot reproduit un défaut de notre rejeu : lecture Vec<u8> refusait SQLite
TEXT, pourtant accepté en octets identiques par le contrat Go/Rust précédent.
Correction générale `event_bytes` TEXT/BLOB partagée par historique et rejeu ;
régression historique conservée et variante TEXT ajoutée au lot signé. L'ordre
historique base/frame/closed/annulation du single apply est explicitement gardé.

`recipe-02` n'a exécuté aucun scénario : inventaire large modifié pendant archive.
La recette locale est maintenant limitée à ses vraies crates identité/enrôlement/
wire et Cargo/config. Le contrôle CLI reste obligatoire séparément, pas supprimé.
`public-cli-01` observe6parcours CLI et1test de diagnostic réussis, mais la recette
reste FAIL :3fichiers étrangers de publication/baseline ont changé pendant la
compilation ; aucune qualification cohérente du parcours public n'en est déduite.

Construction officielle `./swarm.sh build` : Cargo réussit en52,48s, candidat
préparé dev126, mais le collecteur refuse les entrées effectives modifiées
`locales/en.json`, `web/i18n-en.js`, `web/preparation-native.js` du chantier de
préparation actif. Ni installation28/29 ni activation de ce candidat annoncée.
Originaux conservés sous `normal-build-failure/`. Reprise : profil effectif stable
puis rejeu du vrai CLI normal avant activation, Version/notes/assets vérifiés.
Aucun source/processus d'autrui modifié. Aucun commit/push demandé ou effectué.

Réexamen : Prepared permet la reprise exacte tant que l'objet privé reste en
mémoire, y compris après réouverture du Source. Il n'a pas encore de codec de
persistance ; la reprise après crash exige ce raccord et un intent/checkpoint
protégé indépendant. Ne pas présenter la journalisation atomique du compte comme
preuve de ce raccord absent. Suite immédiate30 : codec exact vérifié du proposal,
puis stockage protégé intent/checkpoint et transport/authentification/UX. Compte
réel, humain/bootstrap indépendant, service sous UID séparé, confinement, PA-03,
E2/E3/0sur17 et RETEX global restent ouverts. La boucle autonome continue.


Contrôle des originaux : verify confirme l'intégrité du premier FAIL et des35PASS,
et du FAIL de stabilité public. Il refuse `recipe-02` avec artifact-table-incomplete :
collecte arrêtée avant archive/scellage complets, aucun test. Garder ces originaux
et leur inventaire observé additionnel ; ce dernier ne reconstitue pas les sources
initiales manquantes et ne transforme pas le refus en preuve qualifiée. La revue
28/29 et ses contrôles distinguent explicitement cette acquisition incomplète.
Processus observé après le refus de build : serveur dev125, PID1986387, SHA
2832829115e2d0e8c508a1433db9c6ecf7aaaeef9263da532a93377efbca4d54.
Cette version parallèle ne prouve pas à elle seule la livraison de nos derniers octets.


### Preuve additive de livraison30

Les bibliothèques internes actuelles sont incluses dans le profil normal vérifié
dev128.39tests actuels,6tests du harnais CLI et56assertions UI réelles PASS ; voir
[incrément30](INCREMENT-30.md). Refus historiques/acquisitions incomplètes conservés.
Aucun endpoint personnel, enrôlement humain, confinement OS ni acceptation indépendante
n’est établi.
