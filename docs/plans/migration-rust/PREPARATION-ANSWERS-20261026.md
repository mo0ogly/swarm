# Réponses aux questions du plan — Rust et GLM

## Besoin et frontière

Raccorder l’intention answer-questions déjà émise par le formulaire historique,
via CLI et API protégée, au writer de préparation. La page de préparation complète,
le dialogue fournisseur et la conversion sont des parcours distincts encore ouverts.
Le commit précédent fcda062f6 porte validate-plan. Ce lot conserve ses contrôles.
Biba, SCALING et PAEA restent aux autres propriétaires ; aucun message ni changement
sur leurs sources n’est autorisé par ce lot. Les fichiers partagés reçoivent
seulement les ajouts du parcours annoncé dans PLAN.md.

## Décision de conception

Réutiliser le writer immédiat, son CAS, ses reçus, sa relecture des lignes et sa
reprise de publication plutôt que créer un second writer de réponses. Réutiliser
le validateur ActionPlan avec un mode interne explicite « réponses encore vides » ;
le chemin validate-plan conserve le mode qui exige toutes les réponses.
L’alternative sauvegarde générique reconstruirait la provenance actuelle et pourrait
rendre un ancien plan artificiellement frais : rejetée. La mutation clone le document
et change uniquement texte, hash, révision et date. Le verdict est retiré.

## Contrat et configuration

L’intention lie SHA du plan, préparation/révision et événement. Toutes les questions
sont conservées dans le même ordre, textuellement identiques ; seules leurs réponses
changent. Les réponses partielles restent possibles. Les limites sont les plafonds
persistés du plan v43, du document v42 et des tâches. Aucun prix, délai, nombre de
tentatives ou plafond par réponse n’est introduit. Les constantes version1/JSON et
vocabulaire d’action sont des invariants de protocole, pas des politiques opérateur.
Aucun changement automatique des réglages de l’instance vivante.

## État des preuves

Avant : le scénario construit sa préparation et son plan par opérations publiques,
puis la copie exacte dev94 refuse answer-questions (CLI usage).
L’instance a été livrée en94 par un autre chantier ; cette évolution est conservée.
Une lecture HTTP protégée confirme prepare.html404 et native-capabilities200. Un premier
lancement de recette utilisait un booléen de settings au lieu de sa chaîne :
erreur de méthode corrigée avant de qualifier le reproducer. Originaux conservés.
À cette étape initiale, aucun PASS ni installation n’était établi. Le résultat
final ci-dessous remplace ce statut sans effacer les échecs originaux.

## Améliorations conservées

Le formulaire historique prephase-decisions.js impose maxLength=4000, distinct des
limites autoritaires. Découverte pendant l’inspection, propriétaire préparation UX,
état ouvert : enlever cette politique cachée lors du raccord complet de la page,
avec parcours de saisie longue autorisée par Admin et refus serveur sans perte.
Ce lot ne prétend pas qualifier cette page non exposée par le profil Rust actuel.

Une réponse à un ancien plan doit rester possible sans rafraîchir ses sources ;
le test permanent suit cette propriété et exige un nouveau verdict explicite.
L’apprentissage complète les contrôles d’intégrité du lot précédent, sans modifier
leurs preuves historiques. Le contexte envoyé à GLM et les références Go sont
retenus avec empreintes dans la campagne privée, aucun fichier Go réintroduit.

## Replan de délégation et de livraison

L’appel d’implémentation GLM, contexte67189octets, a reçu HTTP504 à600.09s.
Aucun retry automatique ni plafond de sortie/timeout client. Issue de génération
et consommation inconnues : ne pas affirmer zéro token consommé. Le principal
n’a pas attendu cet appel pour implémenter et vérifier le parcours. Les deux
revues séparées ont également reçu504 à600.097s et600.094s ; la dernière était
limitée au helper/types/encodeur réels (3286octets). Aucune réponse exploitable,
aucun patch GLM intégré et aucune revue externe obtenue. Le code est du principal. Le catalogue SKYNET répond en0.111s et annonce glm-5.2 avec
524288 de contexte ; ni ce nom ni cette limite ne prouvent la version backend.
Hypothèse conservée, owner outillage/SKYNET : évaluer le streaming SSE si le silence
HTTP provoque le délai passerelle ; le service ne fournit ici aucune preuve de
progression ou consommation. Critère : réception de fragments puis réponse complète,
sans perdre les erreurs, empreintes et métriques. Aucun bénéfice établi à ce stade.

Autre reprise : le candidat partagé dev95 a été installé par un chantier concurrent.
Le nouveau test ciblé a rencontré son ancien chemin supprimé. L’empreinte du
processus/bin installé est exactement celle du candidat déjà qualifié ; copie
figée dans la campagne pour la reprise, aucun redémarrage supplémentaire. Les
échecs originaux restent distincts des35assertions du dernier parcours PASS.
Le profil normal qualifié comprend449entrées, sans feature de fixture ni core test.
Les régressions98plan et50écritures passent sur les mêmes octets. La recette est
vérifiée par recipe.py verify. Installation :13assertions (identité, API, ressources,
réglages, missions, Version) et14assertions navigateur Version FR/EN, capture FR relue.

Le diff ciblé passe. Le diff global relève deux lignes blanches finales préexistantes
hors lot (DELIVERY-PREPARATIONS-20261025.md85 et task_validation/managed_receipt.rs293).
Constat conservé avec propriétaire, sans toucher aux sources concurrentes ni
revendiquer une conformité globale. Fichiers persistants/règles ne mettent pas
à jour des agents natifs déjà lancés ; aucun agent natif n’est démarré ici.

## Résultat, RETEX et reprise

**Livré localement dev95, vérification fonctionnelle établie ; revue externe ouverte.**
Le processus, son binaire et les ressources servies correspondent au candidat
normal qualifié SHA ba404609f9f5ddd3060b3b6baa2060922b62ab740c6d02e33ab2775e5c01476c.
35assertions answer-questions,98plan,50écritures,13installation et14Version FR/EN
passent ; ces compteurs décrivent des périmètres distincts. Intégrité de recette
vérifiée. Réglages41 et missions suspendues50/469 inchangés. Aucun provider métier,
identité attestée ou lancement n’est ajouté. Les sources et recettes de rejeu sont
persistantes ; preuves originales sous evidence/20261026-preparation-answers/private.

Les trois échecs GLM et le témoin de métadonnées répondant montrent que réduire
le contexte n’a pas réparé la génération. Cause backend précise et consommation
inconnues ; propriétaire service SKYNET/outillage de délégation. Pas de quatrième
appel inchangé, pas de hausse de budget ni timeout client inventé. Reprise : une
réponse complète de génération sur la route visée puis revue des mêmes octets,
ou décision utilisateur sur un autre parcours de revue. L’hypothèse SSE ci-dessus
reste non vérifiée. La revue du principal et les tests ne sont pas indépendants.
Le critère de revue externe et la qualification globale restent ouverts.

Suite produit conservée : raccord complet de la page existante (incluant ses
services nécessaires et la suppression de maxlength4000 caché), puis conversion
explicite sans lancement, sous politiques courantes, budgets et ordre topologique.
Ne pas transformer des réponses enregistrées en autorisation de mission. Le
présent incrément ne prétend pas terminer cette migration ni cette page.
