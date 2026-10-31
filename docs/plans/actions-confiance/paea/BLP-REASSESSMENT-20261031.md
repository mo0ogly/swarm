# Réexamen des acquis PAEA au regard de BLP — 31 octobre 2026

[English](../../../en/plans/actions-confiance/paea/BLP-REASSESSMENT-20261031.md) ·
[Plan PAEA](../PAEA-APEX.md) · [Plan technique BLP](../BLP-APEX.md) ·
[Relevé des 115 actions](BLP-REASSESSMENT-20261031-ANNEX.md) ·
[Données et empreintes](BLP-REASSESSMENT-20261031.json).

## Résultat et portée

Le réexamen d'applicabilité couvre **115 actions documentées × 7 lots BLP =
805 décisions** : 92 fiches d'incrément (01–93, sauf 82) et 23 actions 94–116
dans le plan principal. Ce nombre mesure la couverture documentaire de la revue,
pas un avancement d'implémentation ou un taux de conformité. Chaque action possède
sa source/section et leur empreinte, les sept décisions, une famille motivée,
les producteurs/consommateurs de référence, les références de versions historiques,
un écart, un propriétaire et une condition de reprise.

Méthode : lecture des fiches, descriptions/limites du journal et registres ;
revue sémantique par responsabilité sur des producteurs/consommateurs courants
représentatifs ; vérification d'intégrité de la recette BLP et comparaison de
ses intrants. Les familles ci-dessous servent à partager les motifs, pas à
déduire une conformité par mots-clés. Cette revue n'est ni une revue de toutes
les fonctions de chaque incrément, ni une nouvelle campagne comportementale,
ni une acceptation indépendante. Les résultats historiques restent inchangés.

**Décision : rouvrir ou maintenir ouverts les critères de confidentialité des
actions applicables. BLP-07 reste ouvert.** Les preuves d'intégrité, d'accord,
d'unicité et de reprise conservent leur valeur dans leur portée d'origine ;
elles n'établissent pas la confidentialité transversale.

## Revue des actions par responsabilité

A = applicable, à reprendre ; P = preuves partielles réutilisables, critère ouvert ;
N = non applicable à la primitive seule, motif explicite ; C = conditionnel à une
sortie de classification inférieure, non résolu avant attribution des classifications
et destinataires. C n'est pas une dispense. L'annexe donne les 805 décisions.

| Famille / actions | Producteur → consommateur examiné | Acquis conservé / reprise exigée | Écart |
|---|---|---|---|
| Contrats hors ligne / 12 | `SourceRef`/canonicalisation → CLI/API de correspondance | Structure et empreinte conservées ; attribution réelle, habilitation et sortie classifiée manquantes ; aucun déclassement dans le calcul pur | BLP-RET-01 |
| UI/API / 17 | Vérificateurs/aperçu moteur → page, réponse, téléchargement | Rendu/session utiles ; vérifier les lecteurs réels, métadonnées et erreurs selon BLP | BLP-RET-05 |
| Laboratoire / 4 | Pilote et effets de fixture → oracles/rapports | Oracles d'effets conservés ; ajouter scénarios de non-divulgation et de déclassification | BLP-RET-03 |
| Identité / 21 | Compte/permission courants → contrôle retenu | Autorité retenue utile ; habilitation BLP et droit distinct de déclassifier à établir ; fixture humaine ≠ humain réel | BLP-RET-04 |
| Stockage privé / 8 | IO à descripteurs → paquets/artefact service | Atomicité, modes et effacement conservés ; classifier copies/lecteurs dans les couches de composition | BLP-RET-02 |
| Frontières Linux / 12 | Sujet noyau/canal → dispatcher | Attribution du pair utile ; elle ne démontre ni flux autorisé ni confinement Internet | BLP-RET-03 |
| Règlement / 8 | Appel gardé → ledger/reçu/stdout | Accord, unicité et reprise conservés ; classifier le contexte, le ledger, les lecteurs et reçus | BLP-RET-01 |
| Git / 16 | Source détenue → pack/import/référence | OLD et idempotence conservés ; le transfert d'objets divulgue avant toute publication de ref | BLP-RET-03 |
| Rapports PA-09 / 2 | Journaux → recomputation/export | Correspondance utile ; protéger l'accès aux identifiants/rapports ; recomputation ≠ confidentialité | BLP-RET-05 |
| Qualification/livraison / 6 | Profil/preuves → déclaration/registre | Preuves anciennes bornées ; requalifier BLP sur candidat exact ; originaux E2 référencés non retrouvés | BLP-RET-06 |
| Accords / 5 | Action/accord moteur → approbateur/dernier usage | Accord Git et révocation conservés ; déclassification spécifique encore absente | BLP-RET-04 |
| Outils/baux / 4 | Preflight/mémoire → départ/reprise | Mutation/bail périmé refusés dans les preuves citées ; lier domaine, sources et politique BLP aux vrais consommateurs | BLP-RET-02 |

Les N ne dispensent pas les appelants : une primitive IO ou de credentials n'a
pas à devenir un second moteur de politique ; le producteur/consommateur protégé
qui la compose reste soumis aux lots BLP applicables. Une écriture de ledger ou
une promotion Git n'accorde aucun droit de déclassifier une copie.

## Écarts, propriétaires et conditions de reprise

Tous les écarts ci-dessous sont **OUVERTS**. « Bloquant » signifie bloquant pour
la revendication BLP correspondante, sans effacer les résultats historiques.

| ID / gravité | Constat et effet | Propriétaire / condition observable de clôture |
|---|---|---|
| BLP-RET-01 / bloquant | `SourceRef.confidentiality_labels` est une représentation non autorisante. Le consommateur `ExecutionDomain` repéré dans le moteur est le diagnostic `blp-check`, explicitement `runtime_authorized:false`. Canonicaliser des labels ne leur donne pas de provenance. | Intégration PAEA/BLP : attribuer versions/octets et politique courants aux sources et destinataires réels ; démontrer refus sans transmission avec témoin autorisé. BLP-01/02/03/04, I03/I13. |
| BLP-RET-02 / bloquant | Domaines fixes et combinaison sont disponibles dans le moniteur pur ; liaison aux contextes, mémoire, compactions, reprises, copies et secrets non établie par les preuves retenues. Les gardes de chemins mémoire ne sont pas une admission BLP. | Lancement/Mémoire/Stockage : tracer chaque contexte exposé et ses dérivés, classifier l'existant explicitement, conserver les restrictions et refuser les anciennes reprises incompatibles ; non-régression Biba. BLP-03/04, I03/I13/I14. |
| BLP-RET-03 / bloquant | Le profil Biba inspecté construit des feuilles lisibles hors racine privée et refuse les sockets AF_UNIX ; il ne fournit pas de contrôle général des destinations Internet. Les permissions d'effet et le protocole pack ne contrôlent pas une divulgation. | Confinement/connecteurs : médiation du sous-ensemble BLP requis, lectures et sorties réelles inventoriées, descendants inclus ; témoin positif puis absence d'octets interdits observée côté récepteur, refus/reprise sans repli. BLP-05, I06/I07/I12/I13. |
| BLP-RET-04 / bloquant | `declassification_available:false` ; les accords existants lient une action Git/effet, pas un droit de baisser une classification. Aucun consentement humain réel à la copie déclassifiée établi. | Identité/accords et commanditaire pour personne réelle : droit spécifique, octets/classification cible/destinataires/révision liés, original conservé, révocation avant émission et substitution refusées. BLP-06, I04/I05/I10/I13/I17. |
| BLP-RET-05 / majeur | Les réponses de l'aperçu exposent proposition/contexte déclarés sous session native ; rapports, téléchargements, erreurs et preuves ne sont pas qualifiés pour plusieurs lecteurs BLP distincts. Aucun exploit sur données réelles n'a été tenté. | UX/API/PA-09 : politique de lecture et de sortie partagée avec CLI/API, sans fuite par valeur, identifiant, erreur, journal ou export ; tests lecteur autorisé/interdit et FR/EN sur binaire exact. BLP-04/05/07, I03/I13. |
| BLP-RET-06 / majeur | `DECLARATION-E2.md` et `TRIAGE-PAEA17.md` référencés sous `claude-e2-arbitrage-20261031T135859Z` non retrouvés par lecture du chemin et recherche ciblée, y compris fichiers ignorés. Cela ne prouve pas leur absence d'une archive non examinée. La clôture PA-10 annoncée reste historique et n'est pas réacceptée ici. | Auteur PAEA/qualification : retrouver originaux/empreintes et profil liés, ou reconstruire une nouvelle preuve explicite sans modifier les anciennes ; aucune clôture BLP-07 par cette annonce. |
| BLP-RET-07 / mineur de traçabilité | Le plan déclare 82 non attribué ; la fiche83 mentionne au contraire une qualification82. Aucun objet82 n'est inventé pour combler le trou. | Propriétaire du journal : localiser la source primaire et ajouter un erratum attribué ; conserver les deux déclarations historiques jusque-là. |
| BLP-RET-08 / majeur de preuve | Recette BLP intègre avec 10+5+14 tests réellement exécutés, mais couverture déclarative sans profil de production complet, UI/API réelles ou oracle réseau. Sur 525 intrants déclarés, seul `tools/verification/recipe.py` diffère au relevé courant. | Qualification BLP : conserver le PASS historique avec son runner original ; examiner l'effet du nouveau runner avant réemploi, compléter les entrées effectives et parcours requis sur le candidat cible. BLP-07 reste partiel. |

## Preuves reprises et limites

`recipe.py verify --evidence docs/plans/actions-confiance/verification-blp/20261031-recipe-run`
retourne `integrity:VERIFIED`, `recorded_status:PASS`. Les sorties originales
confirment 10 tests du moniteur, 5 tests d'intégration et 14 tests de coexistence
Biba ; l'étape `--list` n'est pas un test supplémentaire. Aucune nouvelle exécution
comportementale n'a été nécessaire pour constater ces résultats historiques.
La comparaison des 525 empreintes trouve 524 intrants identiques et le runner
modifié ; la réutilisation reste donc bornée au runner et aux octets conservés,
pas un nouveau PASS sur le checkout entier. Voir
[le relevé de vérification](evidence/blp-reassessment-20261031-01/recipe-verification.json).

Les preuves de revue par action portent sur les références retenues, pas sur
l'intégrité vérifiée de chaque ancien dossier cité. Les versions/empreintes
historiques ne sont pas présentées comme celles du serveur courant. Aucun build,
appel fournisseur, changement de base vivante ou redémarrage n'a été effectué.

### Complément reçu pendant la revue : livraison/UI du socle

Le suivi BLP a changé pendant l'édition des plans : le patch a refusé avant
écriture, puis les nouvelles sections et le RETEX de la session Claude ont été
relus. Les originaux initiaux de la revue restent dans l'archive ; ce complément
ne les remplace pas.

Lecture des artefacts supplémentaires : le record de build282 indique
`VERIFIED_NATIVE_CANDIDATE` et son profil `VERIFIED_DEFAULT_NATIVE_PROFILE`,
`no_fixture_feature:true`, même SHA256 `ea072ccc00fb5ed77ae88a6a53b3d4eba9d406a8d55fedf8e50ed01e3663bbe6`.
`checks.json` du navigateur contient neuf vérifications et zéro erreur. Ces
contrôles portent sur la page, l'exemple sans habilitation, les refus typés,
FR/EN, thème et largeur390 ; ils ne prouvent pas l'administration positive ni
les flux BLP. La session rapporte l'installation282, puis une substitution
concurrente par283 avec note BLP non encore embarquée ; cette revue ne revalide
pas le processus actif et ne présente donc aucun des deux comme image courante
vérifiée ici. Le build seul déclare d'ailleurs `installed:false`.

La preuve supplémentaire réduit l'écart d'observation UI du socle ; elle ne
ferme aucun des écarts de confidentialité réelle. BLP-RET-08 concerne désormais
le complément requis des parcours positifs et des flux réels, la fraîcheur du
profil cible et le réemploi du runner ; pas une absence de build282. Le RETEX
de livraison reste un acquis borné ; BLP-07 transversal garde les dépendances
BLP-01/04/05/06 et les écarts applicables ouverts. Voir
[le complément sourcé](evidence/blp-reassessment-20261031-01/supplement.json).

## Reprise décidée et transmission

Comparer deux routes : répéter toutes les campagnes historiques, ou conserver
leurs portées et cibler les frontières BLP manquantes. Choix de la seconde pour
préserver les preuves utiles et éviter de transformer un grand nombre de tests
d'intégrité en revendication de confidentialité.

1. Intégration BLP : fermer BLP-RET-01, puis les raccords BLP-RET-02 ; une politique
   enregistrée ne suffit pas à les fermer.
2. Confinement/connecteurs : traiter BLP-RET-03 avec les propriétaires actuels,
   sans reprendre leurs fichiers ou processus par surprise ni démarrer AC-02 entier.
3. Identité/UX : BLP-RET-04/05 ; ne pas remplacer l'accord humain requis par un rôle,
   un LLM ou une session opérateur. Les travaux indépendants n'attendent pas l'humain.
4. Qualification : BLP-RET-06/08, puis BLP-07 sur candidat/profil exact et revue
   distincte ; BLP-RET-07 reste une dette de provenance assignée.

Le registre par action et les écarts sont persistés et reliés aux deux plans.
Ils sont prêts pour la reprise GLM/SKYNET choisie par l'utilisateur ; réception
par une session déjà démarrée, décision de son superviseur et effet moteur restent
non vérifiés. Aucun message externe ou lancement n'est réalisé par cet audit.
La revue est achevée dans sa portée documentaire ; les corrections, les contrôles
de confidentialité et l'acceptation globale restent ouverts.

Incrément d'audit/documentation seulement : aucune politique d'exécution, fonction
produit ou issue d'erreur modifiée ; aucun changement de notes de version ou
installation requis. Les politiques/fonctions/erreurs existantes ont été examinées
aux frontières citées, sans revendication d'une revue exhaustive de tout le code.
