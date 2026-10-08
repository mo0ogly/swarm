# Cockpit local stable — 8 octobre 2026

## Résultat

`./swarm.sh start`, `restart`, `status`, `open`, `stop` et `logs` gèrent une instance Linux identifiée. Le redémarrage remplace le serveur et conserve les missions. La configuration locale non versionnée mémorise le projet et l’adresse. Les délais sont des options explicites. Le masquage de `/cifs` reste optionnel et conserve les permissions antérieures.

L’adresse du cockpit est stable. Une connexion expirée affiche un formulaire FR/EN, au lieu d’exiger un lien de session trouvé dans un terminal. Le lanceur ouvre la session privée automatiquement ; les liens permanents des missions ne contiennent plus la clé. Le cookie HttpOnly/SameSite reste valable après redémarrage, pour une durée définie par `config/web-session.json` ou sa surcharge locale. Les contrôles Host, Origin et CSRF restent actifs.

## Vérification réelle

- Sept tests isolés avec serveur natif : instance unique, conservation d’une mission et du cookie après redémarrage, ouverture privée sans affichage de clé, adoption exacte d’une ancienne instance, PID périmé, listener étranger, remplacement atomique et masquage de montages.
- Suite Go complète PASS en 517,310 s ; tests ciblés de connexion, sécurité et durée de session, avec contrôle race. Après les derniers changements de liens/messages et du README anglais, contrôle du manifeste ciblé PASS et tests frontend PASS.
- Tests frontend et catalogue FR/EN, vet, contrat du dépôt et diffcheck.
- Formulaire rendu dans les quatre variantes FR/EN × sombre/État, erreur de clé et focus vérifiés dans le navigateur ; captures dans `screenshots/local-site-20261008/`.
- Sur l’instance historique : arrêt sans agent/revue actif, sauvegarde des fichiers runtime à l’arrêt et du binaire précédent, redémarrage sur le même projet/port. Les dix missions et leurs statuts publics sont identiques avant/après. Un nouvel onglet retrouve la connexion existante et le lien permanent de la mission D.

## Incidents et attribution

La recette du lanceur a reproduit un faux diagnostic de port occupé causé par une sonde bind sur TIME_WAIT ; remplacée par une sonde de listener et une attente de libération. Arrêter seulement le wrapper de montages pouvait laisser le serveur Go vivant : le lanceur suit maintenant le PID réel du serveur. Une ancienne fixture de handler sans Store a révélé un nil dereference ; le chargement de politique accepte ce cas statique.

Une suite complète a également signalé le manifeste historique périmé par les nouveaux fichiers de documentation. Les anciennes preuves ne sont pas réécrites ; `local-web-recovery-manifest.json` lie ce candidat distinct, sans prétendre à une nouvelle acceptation indépendante. Les captures historiques héritées ne prouvent pas le nouveau formulaire.

## Limites

Ce lanceur local Linux n’est pas un gestionnaire de service système ou un déploiement distant. Il ne tue pas des agents ou des processus étrangers et ne supprime pas les données. Le partage CIFS de l’hôte n’est pas réparé. Une clé supprimée ou explicitement renouvelée nécessite une reconnexion. Aucun commit ni push effectué.
