# Résultat — diagnostic des connexions IA

Console implémentée dans la fenêtre IA ; les erreurs de transport affichent une
cause exploitable. Activation locale effectuée après autorisation « ok next » : serveur reconstruit
et relancé via ./swarm.sh restart --no-open, sortie 0. Le fournisseur utilisateur n’a pas été testé par ce correctif.

## Identité et périmètre

Session native, sans identifiant de mission ni acceptation moteur. Base
8923ba5d9874443fb5ec713037135689e7522445 ; candidat non commité, modifications
étrangères préexistantes conservées. Aucun secret dans les rapports.

## Vérifications

| Exigence | Commande / parcours | Résultat | Preuve et limite |
| --- | --- | --- | --- |
| R1 | go test ./... -run 'TestAIConnection' -count=1 | PASS, sortie 0 | DNS, refus TCP, délai, certificat non reconnu via vrai serveur TLS local, redirection refusée |
| R2 | node tests/ai_connections_ui.cjs /tmp/swarm-ai-debug /tmp/swarm-ai-debug-ui | PASS, sortie 0 | vrai serveur Swarm isolé, fournisseur HTTP local, succès puis HTTP 401, deux thèmes et focus console |
| R2 | SWARM_TEST_LANG=en node tests/ai_connections_ui.cjs /tmp/swarm-ai-debug /tmp/swarm-ai-debug-ui-en | PASS, sortie 0 | même parcours en anglais |
| R3 | tests Go et navigateur ci-dessus | PASS | clé et corps d’erreur fournisseur absents du JSON/DOM diagnostic |
| R4 | inspection ai_connections.go et cockpit.js | PASS | serveur 60 s, navigateur 65 s ; attente explicite ; expiration à 60 s non exercée en navigateur |
| concurrence trace | go test -race ./... -run 'TestAIConnection' -count=1 | PASS, sortie 0 | callbacks httptrace protégés par mutex |
| analyse Go | go vet ./... | PASS, sortie 0 | checkout courant |
| frontend | npm test | PASS, sortie 0 | suite npm test et contrôle i18n final après correction des placeholders |
| contrat | python3 tools/agent-workflows/check.py | PASS, sortie 0 | neuf méthodes partagées |
| format | git diff --check | PASS, sortie 0 | checkout courant |

Captures : /tmp/swarm-ai-debug-ui/diagnostic-etat.png et diagnostic-sombre.png,
avec équivalents anglais dans /tmp/swarm-ai-debug-ui-en. Binaire de vérification :
/tmp/swarm-ai-debug. Les fournisseurs HTTP sont des doubles explicitement contrôlés.
La trace arrive à la fin du test ; ce n’est pas un flux d’événements en direct.

## APEX et auto-revue

Analyse : perte de la cause réseau et délai navigateur/serveur limité à 15 s.
Exécution : trace bornée à 64 événements, classification sans erreurs brutes,
console avec textContent et styles à jetons existants. Pas d’en-têtes ni de corps
fournisseur dans la trace, pas de suivi de redirection, pas de désactivation TLS.
Vérification : parcours de succès, refus HTTP, certificat local non reconnu,
conservation de clé et désactivation via la recette existante.
Ajustement : délai navigateur aligné et test anglais corrigé pour préserver l’URL
publique ; placeholders de traduction corrigés après contrôle i18n.
Auto-revue seulement : aucun verdict indépendant revendiqué.

## Limites et activation

Suite Go complète : `go test ./...`, sortie 1 après 540,806 s. Unique échec
rapporté : `TestGraphDeliveryD03QualificationManifest`, `stale input:
ai_connections.go`. Le manifeste de preuve courant référence l’ancienne version
du fichier changé ; aucune preuve historique ni gate modifiée pour forcer un PASS.
La qualification globale reste à renouveler sur le candidat. Les tests ciblés,
go vet, npm test et les parcours navigateur FR/EN passent.
Racine et runtime de l’utilisateur distant inconnus. Le test curl communiqué par
l’utilisateur a réussi, mais la cause de son échec Swarm demeure à observer dans
la nouvelle console. Construire et relancer l’instance concernée puis rouvrir sa
session avant de retester ; ne pas réutiliser une URL de session expirée.

## Activation locale

Instance gérée : /home/fpizzi/workspace/swarm-action-skills, port 18792,
nouveau PID 3624837. ./swarm.sh status confirme running=true et ready=true.
./swarm.sh open a ouvert la nouvelle session, sortie 0. Contrôle HTTP authentifié
des scripts ai-connections.js et cockpit.js ainsi que providers.css : PASS pour
la console et le délai 65 s. Aucun jeton de session affiché ni enregistré ici.
Aucune base de mission modifiée comme fixture ; seul le serveur identifié a reçu
SIGTERM via le lanceur officiel. Le poste distant de l’utilisateur reste hors de
cette activation locale ; qualification globale périmée toujours signalée.

## Copie des logs

Bouton « Copier le diagnostic » sous la console, confirmation accessible et
repli vers sélection manuelle si le presse-papiers est refusé. Parcours réel
navigateur FR/EN avec presse-papiers autorisé : texte relu identique aux logs,
clé absente, sorties 0. Recette : node tests/ai_connections_ui.cjs
/tmp/swarm-ai-copy /tmp/swarm-ai-copy-ui, variante SWARM_TEST_LANG=en.
Le premier essai du harnais avec permission clipboard-write a attendu sans
écriture ; permission de test corrigée en clipboard-sanitized-write.
Contrôle i18n et diff --check : PASS. Instance locale reconstruite et relancée.
