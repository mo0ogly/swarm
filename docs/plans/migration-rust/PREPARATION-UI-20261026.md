# Préparation manuelle Rust — raccord de l’interface existante

## Besoin et périmètre

Le moteur portait déjà les écritures de préparation, mais prepare.html répondait404.
Le lot réutilise son corps HTML, sa palette, ses onglets et son dialogue de décisions.
Un adaptateur natif manuel emploie les routes publiques déjà portées : lecture,
création, sauvegarde besoin/brief/plan, méthode, adoption, réponses, verdict et historique.
Le cockpit conserve langue/thème dans le lien. Pas de clone de frontend ni de Go
réintroduit. Dialogue IA, contexte fournisseur, sources, éditeur enrichi, modèles,
conversion et lancement sont explicitement indisponibles sur cette page.

## Contrats et politiques

La page et ses dépendances sont des ressources exactes embarquées, protégées par
la même autorité session/Host que le cockpit. Le rendu whiteliste lang/theme et
vérifie les identifiants de la source, sans dépendre de l’ordre des attributs.
Les formulaires ne sont activés qu’après la lecture de la session et des réglages.
Le besoin est enregistré explicitement ; aucun délai autosave caché n’est ajouté.
Créer omet la méthode pour laisser le writer utiliser la valeur Admin persistée.

Les plafonds locaux hérités180(titre),4000(réponse),16000(document) et50(historique)
ne limitent pas le parcours natif. Le titre est validé par le writer ; l’interface
compte les octets et affiche la dernière limite Admin relue. Elle ne refuse pas
sur sa copie potentiellement périmée : le writer valide les politiques courantes.
Les réglages sont relus après écriture/actualisation. Un refus32octets puis le
passage Admin à null permettent de conserver et enregistrer52000octets, sans
rechargement destructeur (contre-exemple104 et correction105 conservés). L’historique suit le curseur exclusif et la limite Admin.
Les listes vides ne prouvent pas l’absence de lignes sous un plafond0. Les mutex
UI ready/busy/historyBusy et la révision minimale1 sont des invariants de dialogue
et de représentation, pas de nouveaux plafonds opérateur.

Toutes les mutations sont liées à expected_revision et event_id. Les erreurs IO,
5xx ou JSON incohérent gardent l’événement pour rejeu. Les refus statiques connus
avant mutation libèrent la saisie ; on n’applique pas la suggestion GLM consistant
à déduire « aucune écriture » de n’importe quel statut4xx. Un reçu historique
exige une comparaison/relecture explicite et ne crée pas de troisième événement.
Le succès d’écriture reste distinct d’une erreur de liste postérieure. Les lectures
verrouillent la saisie quand elles remplacent le document ; le texte local reste
présent lors des conflits. Aucune identité humaine vérifiée n’est déduite d’un clic.

## GLM, méthode et limites des revues

La nouvelle hypothèse après trois504 historiques a changé le transport en SSE,
sans retry automatique, délai global client, plafond de sortie ou effort réduit.
Trois réponses complètes ont été reçues : revue du helper précédent, première revue
UI, revue des corrections. Les mesures/usage déclarés et prompts sont conservés.
Ce changement de transport a fonctionné ; sa causalité exclusive face aux504 n’est
pas démontrée. Le modèle demandé/déclaré est glm-5.2, la version backend demeure
non établie. Les revues sont des analyses statiques du contexte fourni, pas des
recettes exécutées par GLM ni une acceptation produit indépendante.

La première revue a identifié des courses/erreurs de reprise corrigées ici.
Deux contre-exemples99 sont conservés : CtrlS écrivait derrière la modale et
refresh effaçait une saisie faite pendant sa lecture. Les tests couvrent après
correction : identité/révision des réponses, sortie incohérente, rejeu après perte
de réponse, conflit CAS, reçu historique puis autre writer et relecture explicite.

La seconde revue ne trouve plus de perte de texte ou duplication d’événement dans
les écritures examinées, mais signale une course du curseur historique. Reproduite
sur101, elle est corrigée par sérialisation de la lecture. Ses autres remarques
sont qualifiées : refresh sans préparation est dans une section cachée (pas de
parcours utilisateur atteint) ; la perte de focus des décisions n’est pas reproduite,
Escape restaure bien le bouton dans Chrome. Un autre défaut de focus, créé pendant
le raccord historique, est toutefois reproduit sur103 : la fermeture différée
reprend le focus malgré un choix plus récent. Le correctif ne restaure l’ancien
bouton que si aucun nouveau contrôle n’a acquis le focus.

## Échecs de méthode et préconditions corrigées

Les premiers essais incomplets de navigation sont conservés : le test n’ouvrait
pas l’accordéon du menu ; il attendait aussi une révision peinte avant la fin de
l’opération et cliquait un bouton encore désactivé. Le critère attend désormais
les contrôles réellement interactifs. Les navigateurs/serveurs arrêtés étaient
uniquement ceux des fixtures identifiées ; aucun timeout global n’a été ajouté.

Une construction100 est refusée après un commit PAEA concurrent : profil compilé
normal conservé, mais métadonnées Git devenues anciennes. Un autre candidat100
révèle une hypothèse fausse de notre garde HTML (class avant id sur le bouton) :
HTTP400 reproduit, garde corrigée sur les IDs, assertion HTTP200 avant attente de
la page. Ces candidats ne sont pas présentés comme livraisons acceptées.

Des assertions de recette trop larges sont corrigées avec leurs originaux : elles
incluaient les appels du cockpit avant l’entrée dans la préparation, puis cherchaient
le plan dans l’onglet Besoin après rechargement. La fixture capture maintenant
la surface et le contenu effectif saisi. Un ancien numéro de révision dans une
assertion ajoutée a aussi nécessité l’arrêt de la seule fixture concernée avant
reprise avec la révision issue du vrai producteur. Aucun état n’a été changé pour
faire passer un test ; les résultats échoués ne sont pas réécrits.

## Transmission de l’amélioration SKYNET

La skill personnelle dispose de --stream : fragments privés/progression, modèle
et usage déclarés, texte final/stop/DONE requis, sortie partielle refusée.13tests
HTTP hors ligne couvrent aussi le mode historique, refus, EOF, outils, redaction,
cap omis et reprise. Trois traces SSE réelles sont rejouées exactement dans le
parseur canonique. La première tentative a révélé une différence de cadrage :
le prototype arrête sa capture à la ligne DONE avant le séparateur vide suivant.
Le helper reconnaît la sentinelle terminale complète sur sa ligne, mais refuse
un EOF sans sentinelle. Ce cas a sa régression maintenue dans la skill.

La copie Dremio était encore celle du24octobre et prescrivait un plafond et un
effort réduit contraires à la décision du25. Ses trois originaux sont préservés,
la copie est synchronisée avec la skill canonique et reste ignorée par Git.
Les rôles déjà démarrés ne reçoivent pas implicitement ce nouveau contexte.
La skill reste locale ; aucun proxy, modèle ou service tiers n’a été reconfiguré.

## État final

Installation107 vérifiée, SHA b05390d2a1b12e5f856fbd34b615b1dff4a04629169c88e4e4f9d8cc0bb17b1f ;
la correction de traduction108 fait l’objet d’une recette/livraison distincte.
La livraison106 avait été vérifiée puis remplacée par une livraison concurrente107.
Le premier contrôle contre106 refuse cette différence ; l’inventaire effectif107
et les octets du processus sont ensuite vérifiés, sans attribuer106 à107.
16contrôles HTTP/identité/ressources,14assertions Version FR/EN et6assertions
lecture de la préparation passent. Réglages schema41 et missions suspendues
révisions50/469 sont conservés. Leur activation Admin reste explicite.

La recette navigateur permanente couvre101assertions sur le binaire installé107 :
FR/EN, etat/sombre, desktop/mobile, vraies saisies, refus, pertes de réponse,
rejeu exact, CAS, focus, pagination et changement Admin. Les35assertions CLI/API
réponses au plan passent aussi sur105 ; leur portée reste celle des producteurs
inchangés, sans les présenter comme un nouveau test de107.
Le résultat de cette recette107 et son intégrité sont consignés dans les preuves.
Les captures de la page installée et du parcours sont inspectées.

Les trois revues GLM SSE ont abouti (usage déclaré total111577tokens), sans
prétendre une économie mesurée ou une acceptation indépendante du delta final.
Les derniers correctifs focus/politique sont revus et reproduits par Codex.
13tests HTTP hors ligne et3rejeux réels vérifient le helper SSE canonique.

RETEX : distinguer candidat préparé, processus installé et binaire testé ; l’alias
local-build.json peut changer pendant le travail d’une autre conversation.
Capturer le SHA du processus puis retrouver son inventaire est la référence.
Deux enchaînements de vérification ont continué après une assertion échouée :
leurs sorties sont conservées sans valeur de PASS ; les reprises dépendantes
emploient désormais set -e et des étapes séparées. Un échec de script ne doit
jamais déclencher une recette sur un chemin de binaire non établi.

La livraison est bornée à la préparation manuelle. Dialogue IA, conversion,
lancement et qualification globale de migration restent ouverts ; owner Codex,
reprise par un lot vertical avec politiques courantes et critères explicites.
Aucun commit/push, appel fournisseur métier ni acceptation utilisateur implicite.
Voir [preuves et revue](evidence/20261026-preparation-ui/README.md).

## Replan de traduction après inspection visuelle

La capture anglaise107 montrait « Connecté » : clé absente du catalogue anglais.
Une assertion navigateur FR/EN reproduit le défaut avant correction. La clé est
ajoutée au catalogue source et la ressource embarquée est régénérée ; la recette
finale doit désormais compter105assertions. Cette correction conserve le statut
fonctionnel et n’introduit aucun paramètre opérateur ou traitement d’erreur.

La construction108 initiale a été refusée par native-build-git-observation-changed.
Le diff avant/après établit la cause exacte : création par Codex du dossier public
de preuves durant la compilation, HEAD inchangé. La reprise garde le dossier en
place et ne modifie aucun fichier jusqu’à la fin du build. Ce refus de notre
méthode ne doit pas être attribué au chantier concurrent. Le profil et l’échec
original restent conservés ; aucune protection n’a été désactivée.

## Clôture du lot manuel — livraison108

Le binaire v0.1.1-dev.108 est installé par swarm.sh build/restart et identifié
par le processus réel, la version HTTP et472entrées effectives du profil normal.
SHA256 : 5ef899642fae0a8a7fb2048dfa458a17a0cb5f06496255ffd22cdcb7231384ac.
105assertions navigateur PASS sur exactement ce binaire ; intégrité de recette
VERIFIED.16contrôles installés,14Version FR/EN et6lecture préparation PASS.
La capture anglaise finale montre Connected ; aucun dépassement mobile observé.
Les preuves107/101assertions restent historiques. Réglages41 et missions pause
50/469 restent inchangés ; leur configuration n’est pas une fixture de tests.

Revue finale limitée au delta manuel : politiques, contrats des fonctions,
erreurs, documentation et octets embarqués. Deux blancs finaux hors périmètre
restent signalés par git diff --check global ; le diff ciblé passe. Leurs owners
respectifs conservent leur chantier, aucune conformité globale n’est déclarée.
Aucun nouveau droit ni plafond, aucun secret publié, aucun commit/push réalisé.
