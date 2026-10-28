# PAEA — incrément 18 : reprise des contributions GLM

## Besoin, état et décision

Le 26 octobre 2026, après le commit `93848b3` des incréments 16/17,
l’utilisateur demande « reprends glm ». La reprise porte sur PAEA dans ce chat.
Codex racine garde l’intégration ; une contribution GLM interne bornée examine
la résolution de facture, ses entrées strictes, sa route HTTP et son interface.
Aucune délégation native Swarm ni reprise de mission suspendue.

L’instance observée au départ est dev87, PID 70398, empreinte
`d98fbaa4aa6f65c0a80bfbb06bc3515f602ed84b774c516a9579c31c07d05446`.
Elle conserve son identité de livraison historique ; le commit ne l’a pas
reconstruite. IA3/AC-02 et les autorités réelles restent des prérequis ouverts.

Options : avancer sur la frontière protégée sans ces remises (rejeté), ou
qualifier les défauts reproductibles du parcours actuel puis les corriger dans
ses contrats autorisés (retenu). La revue transmet onze sources complètes,
producteurs/validateurs et tests compris, avec empreintes. Le contexte a été
vérifié sans clé connue ; les fixtures sont fictives. L’appel direct SKYNET
utilise l’identifiant API `glm-5.2` ; ni cet identifiant ni l’alias local
`glm-5.3-flash` ne démontrent la version réelle du backend. Pas de plafond de
sortie, effort réduit, retry ni watchdog global ajoutés.

## Critères et preuves

1. Conserver la réponse, son état de transport et l’usage déclaré ; distinguer
   réponse complète, proposition utile, correction appliquée et effet vérifié.
2. Pour chaque défaut, témoin valide puis reproduction minimale avant correction,
   ou réfutation motivée/contexte manquant. Aucune garantie protégée inventée.
3. Tout correctif justifié inclut cas de refus/reprise, recette permanente,
   documentation FR/EN, revue des politiques/fonctions/erreurs liée aux octets et
   installation locale par le chemin supporté si le produit change.
4. Préserver les travaux/index étrangers ; aucune publication supplémentaire
   déduite de la seule reprise GLM.

État initial : contribution en cours. Preuves locales privées sous
`evidence/20261026-glm18-g__c6d8j/` ; leur existence dans un clone n’est pas promise.
L’avis GLM ne constitue ni exécution, ni revue d’acceptation indépendante.
**0/17 invariants de bout en bout** ; E2/E3 et RETEX final du programme ouverts.

## Résultat de la reprise et RETEX

La réponse GLM est complète (`stop`), après 419,328 s : identifiant demandé et
déclaré `glm-5.2`, 24 835 jetons d’entrée et 47 504 de sortie déclarés, dont
45 449 de raisonnement ; aucun tarif ni coût monétaire déduit. Aucun plafond
client ou effort réduit n’était envoyé. Le modèle n’établit aucun défaut
reproductible du produit. Son hypothèse de réflexion de données JSON nécessitait
un producteur absent du paquet : `HttpServiceError` se trouve en réalité dans
`service.rs`, pas dans le chemin supposé `service/mod.rs`. Display/Debug exposent
le code sûr ; `source()` garde la cause JSON typée. Les tests ci-dessous
vérifient ce contrat plutôt que d’inventer un correctif. L’indication GLM d’une
version minimale de Chrome n’est pas adoptée : les capacités sont observées sur
Node v22.23.2 / Google Chrome 130.0.6723.116 réellement utilisés.

| Critère | Preuve et résultat observé |
|---|---|
| Erreurs privées et doublons divergents | Nouveau test HTTP exact PASS : deux routes × cinq mutations ; cause JSON contenant le canary pour les mauvais types/champs, Display/Debug/corps sûrs ; versions divergentes dans les deux ordres et clé échappée refusées ; témoins, reprise et Root inchangé. |
| i64/BOM purs | Nouveau test exact PASS : contrôles i64 max, dépassement côté registre/demande, BOM côté registre/demande, cause JSON conservée et rapport de reprise identique. |
| Transport réel du navigateur | `encoding-recipe-01` PASS : 28 assertions sur le binaire normal dev91 figé, une configuration FR/clair/1440, trois vrais refus attendus, zéro diagnostic inattendu ; BOM des deux fichiers observé dans les POST du Root isolé, refus puis reprise. Script/page PAEA servis identiques aux sources courantes. |
| Intégrité des recettes | `rust-recipe-03` et `encoding-recipe-01` : VERIFIED/PASS ; première exécution Rust conservée et VERIFIED/FAIL. |
| Inventaires réutilisables | Quatre manifestes Rust PAEA corrigés ; précontrôle des includes littéraux sans manquant. Documents de commandes, familles de fixtures et ressources web imbriquées déclarés ; cela ne remplace pas un graphe de compilation effectif. |

La première recette Rust a exécuté son test avec succès mais refusé la clôture :
`version_history.json` a changé pendant la compilation. Le second test n’avait
pas démarré. La préparation d’instantané a ensuite refusé dix includes non
déclarés avant toute copie/compilation. Les inventaires concernés ont été
complétés puis contrôlés ; un seul instantané figé de 905 fichiers déclarés
(`.cache/paea-glm18-g__c6d8j-02`, cache Cargo lié explicitement) a exécuté les
deux tests exacts, sources stables. Aucun FAIL n’est réécrit ni présenté PASS.
Les recettes complètes attendent désormais cinq tests HTTP, sept tests de
registre et 38 refus navigateur ; ces suites complètes ne sont pas rejouées
ni annoncées passantes dans cet incrément. Le mode d’encodage est explicitement
distinct du mode complet FR/EN/thèmes/tailles. Un ajout final au seul commentaire
du helper décrit ses préconditions/erreurs ; sa substitution inverse reproduit
les octets testés, sans changement exécutable (`documentation-only-test-delta.json`).

L’instance a évolué concurremment de dev87 à **dev91**, PID 235185 observé,
SHA256 `2ed1820f751ed9bed45d4269b556ccbc35a4f86799c128f9acc142df5c1c8583`.
Le collecteur `rust-1vmjo2jh` qualifie son profil natif normal ; le navigateur
utilise le binaire immuable portant cette empreinte. Cette livraison étrangère
n’est pas attribuée à la reprise GLM. Aucun octet produit n’est modifié ici :
seuls tests, recettes et documentation changent. Aucune note de version produit,
construction/réinstallation, interruption du service ou nouvelle publication
n’est donc nécessaire pour cette évolution de vérification. Les missions
vécues et leurs données ne servent jamais de fixture.

Apprentissage : figer les entrées lorsqu’un catalogue partagé évolue ; vérifier
les includes avant Cargo, sans supprimer une dépendance pour obtenir PASS. Le
paquet GLM doit nommer et fournir les vrais producteurs d’erreurs ; son avis reste
une proposition. Le client non streaming ne rendait pas l’avancement observable :
la consultation `/models` était accessible en 0,097 s mais ne prouvait pas la
progression du modèle. Amélioration conservée pour le mainteneur SKYNET : mode
streaming explicite avec preuves SSE/annulation/erreurs/usage, à qualifier avant
emploi. Elle n’est pas réalisée et n’autorise aucun appel ou suivi supplémentaire.

Cette tranche de revue/régressions est vérifiée, sans correction produit inventée.
Revue de même session ; pas d’acceptation indépendante, registre protégé, accord
ou règlement ajouté. IA3/AC-02 et autorités réelles conditionnent toujours la
suite protégée. **0/17 invariants de bout en bout** ; E2/E3 et RETEX final du
programme restent ouverts. Après ces vérifications, l’utilisateur demande le commit de cet incrément.
Le commit porte sur les tests, recettes et documents PAEA ; les preuves privées
restent locales et les passages étrangers du README sont exclus. Aucun push
ou changement de livraison locale n’est inclus dans cette opération Git.
