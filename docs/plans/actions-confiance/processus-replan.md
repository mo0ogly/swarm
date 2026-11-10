# T5 / R15 — définitions publiées et dossiers versionnés

Reprise du 15 octobre 2026, propriétaire Codex. Réalisation directe autorisée,
sans agents natifs, compte tiers, paiement réel ni acceptation de mission simulée.
Méthodes : replan, spec-builder et swarm-model-design ; mise en œuvre bornée APEX.

## Besoin et état observé

Une équipe doit publier son parcours, traiter plusieurs dossiers sur une version
précise et comprendre qui a validé ou accordé une exception. `Work` représente
missions/tâches/événements de développement ; il ne porte pas ces objets métier.
Les recherches de producteurs process-definition-v1/case-v1 et l’inventaire des
sources n’ont trouvé que processus d’agents et recettes de coordination. Conserver
ce moteur ; raccorder un service métier distinct à l’identité et aux preuves T2–T4.

Options : transformer Work en dossier (mélange des rôles, états et UX) ou ajouter
un service cohésif sous le moniteur de référence existant. Retenir le second.
Le stockage atomique Actions de confiance est réemployé pour la première tranche :
identité, configuration, mutation de dossier et preuve dans le même verrou et la
même publication. Sa capacité technique existante de8Mio est exposée ; aucune
rétention, suppression ou limite métier cachée pour forcer une admission.

## Contrats de la première tranche

- `process-definition-v1` : identifiant, version positive, empreinte canonique,
  libellés FR/EN, nœuds, départ et transitions. Publication par admin I3 réel ;
  version nouvelle et révision attendue, ancien contenu immuable. Pas de commande
  arbitraire, URL, script ou champ inventant une autorisation. Graphe référencé,
  départ/fin accessibles ; cycles ordinaires refusés, reprise par exception nommée.
- `case-v1` : identifiant, définition/version/digest fixés, propriétaire authentifié,
  participants locaux actifs, ressource classée, titre et référence non sensibles,
  digest d’entrée immuable, nœud courant, révision, historique attribué. Création
  seulement sur dernière version explicitement attendue ; publier v2 ne déplace
  jamais les dossiers v1. Aucun fichier ni secret brut admis.
- Transition : event_id, dossier/révision/digests exacts et transition nommée ;
  garde de source, rôle/intégrité locaux, participation et séparation. Validation
  par approbateur I2 distinct du demandeur si la politique commune l’exige ;
  exception définie par admin I3, motif explicite et historique conservé. Aucun
  saut arbitraire, mutation inter-dossier ou autorité fournie par le client.
- Rejeu lié au contenu et à l’autorité actuelle ; conflit sans mutation. Les
  refus d’admission ne capturent pas l’entrée. Preuves et originaux canoniques
  seulement après contrôle ; contexte identity-context-v1 lié aux nouvelles preuves.
- Admission T4 : nouveau chemin local-process-state/swarm/process-workflow,
  exclusivement public/internal, ressource exacte administrée et contexte concordant.
  Réseau fournisseur, pièces jointes et données sensibles restent refusés.
- Politique `process_policy.enabled` persistée/éditable Admin, booléen strict,
  défaut inactif, omission conserve. Bornes de transport/représentation distinctes.
- Go/API/CLI : namespace `process`, API `/api/v1/processes`, client distant existant
  mutualisé ; contrôle de session/CSRF opérateur et Bearer métier. Introspection
  service avant le verrou, revalidation commune sous verrou ; aucun repli local.

Une transition suit un dossier ; elle ne crée pas une approbation de paiement ni
un engagement T2. Les effets et la migration/retraite explicite de versions font
l’objet des extensions suivantes avant qualification globale. Aucun modèle de
fixture ne sera présenté comme adopté par l’utilisateur.

## Recette et séquence

| Étape | Critère observable | Contrôle |
| --- | --- | --- |
| Contrat initial | namespace absent constaté, schéma attendu | test CLI nommé avant code |
| Définitions | publication v1/v2, contenu ancien préservé, graphe invalide refusé | Go et API/CLI isolés |
| Dossiers | v1 conservée après v2 ; autre propriétaire refusé ; attente de rôle visible | refus sans mutation et reprise |
| Validation/exception | auto-validation et saut refusés ; accord exact et exception attribués | rôles/groupes/digests, concurrence/race |
| Reprise | rejeu exact, conflit, réouverture et identité révoquée | racines isolées, contexte authentifié |
| UX propre | liste des dossiers, états vides/erreurs/résultat, conception/version | FR/EN, thèmes, mobile, clavier et console |
| Build séparé | artefact Processus sans scripts ou écrans Projets | construction et navigation isolées |

Frontière frontend : entrée/navigation/style applicatifs propres ; seulement
jetons, logo et i18n partagés. Publication/paramètres en dialogues accessibles.
Admin et JSON avancé restent secondaires aux dossiers. Aucun pourcentage fictif.
Documentation opérationnelle FR/EN et recette du résultat incluses. Les contrôles
R13 vérifiés sont réutilisés tant que leurs branches/entrées pertinentes restent
inchangées. Toute erreur relance le diagnostic ; aucun original n’est effacé.

Incident de compilation initiale : le marqueur `trustIdentityDisabled` a été
composé avec un résultat booléen de rôle sans conversion explicite. Original
`process-first-race.out` conservé, zéro scénario exécuté. Conversion corrigée au
contrôle d’admission des participants ; relance sous un autre reçu, pas de PASS
attribué à la tentative de compilation échouée.

Incident T5-replay-data reproduit : publication/version fixée passent, mais le
rejeu de création rend encore le dossier après désactivation de l’admission.
Cause : résultat historique consulté avant la garde T4. Correction générale :
autorité, participation et admission courantes avant tout rejeu, sans réécrire
l’événement historique. Original `process-first-typed-race.out` conservé ; garde
`ReplayRechecksDataAdmission` à rejouer.

Vérification de la garde de rejeu : `process-replay-fixed-race` puis
`process-guards-typed-race` passent. Le second reçu couvre 23 cas nommés, dont
versions, refus sans mutation, graphe, politique, rejeu, exception, reprise,
participation, concurrence et namespace distant. Une erreur de conversion du
résultat struct de politique vers pointeur a empêché la compilation de
`process-guards-final-race` ; zéro scénario, original conservé. Conversion
explicite corrigée, gardes exécutées dans le reçu suivant.

La première recette navigateur révèle un remplacement double du détail après
transition : le rafraîchissement de l’inbox le rendait, puis une seconde lecture
remplaçait de nouveau ses boutons. Original `process-browser-first` conservé.
La transition utilise désormais un seul rendu courant ; la recette garde le clic
immédiat suivant, sans attente arbitraire ni contournement du problème. Les
réponses arrivant après un changement d’identité sont aussi rejetées par le
client ; aucune identité précédente ne peut repeupler l’écran courant.

Raison de l’extension ciblée de non-régression : sérialisation/lecture de l’état
commun, inventaire T4 et routage du client distant ont changé. Relancer seulement
T2, T4 admission/client et contexte/introspection T3 ; les autres tranches ne sont
pas réimplémentées. Les sources Go restent figées pendant ces groupes.

Résultat final des gardes T5 : 23 cas nommés sous race dans
`process-guards-typed-race.json` ; aucun des reçus de compilation échouée n’est
additionné. Les groupes de non-régression finalisés couvrent126 T2,68 admission
et client T4,60 contexte/introspection T3, tous code0 et sources Go inchangées.
Ce sont277 cas distincts en groupes, pas une exécution unique277. La recette
publique du pilote passe sept scénarios et23 contrôles navigateur ; la recette
Processus finale en passe18, dont exception CLI distante, données non fiables
rendu texte et absence de débordement. Son nom historique `content-before`
correspond à un PASS : le composant partagé gérait déjà le retour à la ligne,
aucun correctif de palette ou échappement supplémentaire n’a été nécessaire.

Le premier manifeste a été généré avant la fin des extensions de recette.
`process-manifest-final-race` refuse effectivement le fichier de recette périmé.
Cause de méthode, propriétaire Codex : figer recette et docs avant génération,
puis tester le résultat généré. Reçu original conservé ; génération1424 sources
et `process-manifest-documented-race` final passent. Un lecteur ad hoc a ensuite
supposé que `assertions` était une liste ; le producteur réel expose un entier23.
Lecture corrigée, sans modification de recette ou moteur pour cette erreur.
La recherche de fichiers temporaires est désormais bornée aux préfixes détenus,
au lieu de parcourir les répertoires privés système sans rapport.

Auto-revue avec code-reviewer : contrôle des producteurs/parseur/persistance,
ACL et rôles courants, sérialisation, rejeux, source de l’horloge, écriture atomique,
réseau hors verrou, contexte de preuve et rendu textuel. Aucune revue indépendante
ni acceptation de mission n’est fabriquée. Les recettes portent sur un émetteur
local et des données public/internal. Le lien d’effet T2, le cycle d’archivage des
versions et la qualification globale restent ouverts. L’UX locale à jeton mémoire
ne promet pas un SSO inter-origines autonome.

Extension finale de recette : l’architecture R10 cite cinq largeurs. La recette
Processus vérifie maintenant les20 combinaisons FR/EN × etat/sombre ×
1440/1024/760/390/320px en lecture, sans réexécuter les transitions.38 contrôles
navigateur au total et zéro diagnostic inattendu dans
`process-browser-responsive-final` et la recette publique T5 finale. Le wrapper
`tests/actions_confiance_t5.py` respecte le protocole existant, mais n’active aucune
mission : ses quatre scénarios publics CLI/navigateur passent, avec50 assertions.
Les critères isolés1–4 exécutent leurs préconditions, pas un ancien reçu rejoué.
Au total61 contrôles navigateur sur le binaire final,23 pilote et38 Processus.

Le lien d’effet et la migration/retraite de versions sont des extensions possibles,
pas des critères supplémentaires inventés pour empêcher le passage à T6 : les
critères locaux initiaux R15 sont démontrés. L’acceptation indépendante et la recette
globale restent séparées. Prochaine tranche : inventaire R16–R19 dans les producteurs
réels de signature/ancrage/export, des simulations et connecteurs ; conserver le
pilote et ses preuves v2 pendant toute évolution. R14 sensible reste ouvert.
