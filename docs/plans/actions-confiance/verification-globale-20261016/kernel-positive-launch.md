# Lancement positif R20 à examiner

Préparation du16octobre2026. Propriétaire : Codex ; autorité environnement : utilisateur/administrateur Linux. Aucun lancement noyau effectué par ce dossier.

## Candidat et portée

Binaire `bin/swarm-actions-confiance-secret-data-qualified`, SHA256 `7de055bcfff8b030828aab87b7ed8744a1a1cffeee2d32f458098e1105da0e79`. Le diagnostic existant n’accepte qu’un corps vide et crée son helper fixe ; aucun PID, commande, chemin, URL ou programme BPF sélectionné. Trois sondes fixes sur son thread principal seulement. L’opération ne lance aucune mission enregistrée.

Première étape proposée : fixture neuve, politique kernel_check active de17secondes/ring131072octets, administrateur local de fixture, appel unique `trust kernel-check`. Les valeurs non standard sont une entrée explicite de recette, pas un délai codé dans Swarm. Le compte rendu conserve politique, identité PID/démarrage, empreinte du binaire et octets métier avant/après.

## Précondition et décision réellement manquante

Ici, le ring a refusé EPERM avant programme/perf ; cause unique non établie. Aucune nouvelle sonde identique. Comparaison : modifier les privilèges de l’application servie élargirait le périmètre ; un environnement Linux de test séparé, explicitement autorisé, permet de limiter l’essai au binaire et à sa fixture. Cette seconde option est retenue pour la décision humaine. Le dossier ne crée ni VM, ni capacité, ni règle sudo/sysctl/memlock.

L’administrateur doit fournir un environnement qui autorise chargement BPF et perf sur les seuls processus possédés, et l’accès aux identifiants des trois tracepoints. Les restrictions restent à constater dans cet environnement ; des capacités nominales seules ne prouvent pas le chargement. La documentation primaire [perf security](https://docs.kernel.org/admin-guide/perf-security.html) distingue les permissions perf et les ressources ; le [ring buffer](https://docs.kernel.org/bpf/ringbuf.html) peut perdre des émissions. Ces sources ne prouvent pas la cause d’EPERM ici.

## Assertions du premier lancement

Sortie0 et available/fixture_verified/cleanup_verified vrais ; source ebpf, portée owned_fixture_only, exactement un PID possédé ; catégories process/file/network réellement présentes ; identité/nonce cohérents, ordre et pertes explicites, aucun contenu brut ; aucun PID appartenant à un autre programme. État métier inchangé et helper terminé après réponse. Une réponse vide, code3, perte ou nettoyage échoué ne vaut pas réussite positive. Pas de retry automatique.

## R20 restant après ce premier succès éventuel

Le diagnostic ne prouve pas l’attribution mission/tâche/tentative du collecteur métier. Restent : fixture enregistrée et charge connue, seconde fixture témoin possédée mais non ciblée, exclusion réelle de ses événements ; pertes par saturation bornée et compteurs ; interruption/redémarrage/nettoyage réels ; mesures appariées même charge sans/avec collecte, temps monotone et CPU, résultats bruts et dispersion, sans seuil inventé. Aucun processus extérieur au projet n’est utilisé comme témoin. Chaque variante doit être préparée et autorisée dans cette portée avant exécution.

Aucune qualification globale, revue indépendante, acceptation ou RETEX final ne découle de cette préparation.
