# APEX — diagnostic des connexions IA

Demande : console de diagnostic en bas de la fenêtre IA et remplacement de
« Connexion impossible » par une cause exploitable. Implémentation native autorisée.
Base inspectée : 8923ba5d9874443fb5ec713037135689e7522445, checkout déjà modifié.
Les modifications étrangères sont conservées ; aucune mission ni base active utilisée.

## Analyse et décision

Le client HTTP serveur masque toute erreur de transport. Le test serveur et le
navigateur imposent 15 s, contre 60 s pour les appels normaux. CORS ne concerne
pas le trajet serveur Swarm vers le fournisseur.

Deux options : afficher uniquement une catégorie d’erreur, ou ajouter une trace
HTTP structurée au test. La trace est retenue : elle donne le dernier stade atteint
sans exposer erreurs brutes, en-têtes, corps fournisseur ni identifiants proxy.
Les appels normaux gardent leur contrat texte ; seule la réponse du test est enrichie.
La console affiche l’attente immédiatement, puis la trace au retour ; pas de streaming.

## Séquence et exigences

| ID | Effet | Responsable / fichiers | Vérification | Reprise |
| --- | --- | --- | --- | --- |
| R1 | Cause DNS/TCP/TLS/délai/redirection identifiable | Implémentation : ai_connection_debug.go, ai_connections.go | tests Go réseau et TLS réel local | conserver échec, diagnostiquer catégorie |
| R2 | Console en bas, URL appelée, étapes, durée et HTTP | Implémentation : web/ai-connections.js, providers.css | navigateur contre fournisseur local 200/401, thèmes FR/EN, focus clavier | distinguer échec navigateur et serveur |
| R3 | Aucune clé ni corps d’erreur fournisseur dans la trace | Implémentation et auto-revue | assertions sur JSON diagnostic et DOM | ne jamais afficher erreur transport brute |
| R4 | Test serveur 60 s, navigateur 65 s | Implémentation : cockpit.js, ai_connections.go | inspection et parcours public | délai explicite, aucune relance automatique |

Vérification après implémentation via verify-fix ; auto-revue via code-reviewer.
Pas de verdict indépendant revendiqué, ni d’acceptation moteur.
Livraison : source et binaire de vérification isolé ; aucun redémarrage de mission.
