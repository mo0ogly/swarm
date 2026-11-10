# RETEX intermédiaire G05.2 — pas une clôture globale

Le besoin était de réutiliser la couche identité existante pour Biba Rust et
rendre son état visible dans l’Admin. Résultat : consommateur natif effectif,
mapping sans jeton de secours, contexte de preuve canonique ; candidat217
installé et lu. Le transport HTTP commun demeure ouvert, source protégée et pin
non configurés ; aucune identité réelle n’a été inventée.

Preuves distinctes :2tests Root et11gouvernance sous Cargo ;10startup profil
normal ;13startup sous feature fixture (recouvrement, pas addition) ;1test SDK
financier complémentaire ;27CLI/API,42navigateur sur candidat ;14Version et34lecture
Admin installée sans POST. Les images FR mobile sombre et EN bureau clair ont
été inspectées. Pas d’acceptation indépendante ou de qualification globale.

Deux appels GLM complets ont aidé à examiner le contrat. Leurs défauts supposés
ont été contrôlés sur les producteurs : Pins.validate couvre déjà le digest,
with_active ne consomme pas de Head, et l’enum ne permet pas export/show/anchor.
Les paquets incomplets et extraits par lignes ont alimenté de mauvaises hypothèses.
Owner Codex : relire les types/gardes de chaque paquet, même après un autre appel.
118028tokens déclarés sur deux appels ; tarif/coût non établis. Pas de retry réseau.

Le compteur lexical13 a produit un FAIL de recette malgré10tests réels verts.
Original conservé, lecteur de reprise vérifie les noms/sources, sans relance ni
réécriture du FAIL. Le profil fixture séparé établit ensuite les13cas. L’attente
Cargo partagée n’a pas exécuté de test ; seul notre Cargo a été retiré. Le test
SDK réutilise des bibliothèques déclarées par le profil avec empreintes actuelles,
sans inventer une empreinte historique absente. Une ambiguïté base64 a été refusée
avant rustc puis résolue par la version réellement épinglée.

La concurrence a ajouté mission_discovery puis extrait une garde PAEA commune.
Les sources retenues ont distingué snapshot livré et checkout ; le contrôle de
continuité a refusé les différences au lieu d’en déduire un résultat inchangé.
Lecture sémantique puis10tests normaux+anti-JSON ont vérifié la garde extraite.
Aucune mission native, commit, compte tiers, paiement ou nouveau privilège.

Notre méthode a encore espacé des points au-delà de60s pendant l’analyse interne.
Le découpage des outils seul n’a pas réparé ce défaut. Owner Codex, amélioration
ouverte : borner aussi chaque décision/rédaction et mesurer chaque intervalle ;
aucun réveil ou simple rappel ne prouve sa correction. G05.3 et RETEX final restent
ouverts. Prochaine expérience : source protégée et pin identifiés, puis contrat
startup/transport public et tests de refus sans adoption d’un principal JSON.
