# R14 — état au16octobre2026,05:16UTC

Propriétaire Codex. Reçu courant :
[secret-data-verification.json](verification-sensitive-20261016/secret-data-verification.json).
Contrôle local vérifié, aucune revue indépendante ni acceptation globale.

| Critère T4 du plan initial | Résultat observé | Limite |
| --- | --- | --- |
| Flux contrôlés au point d’envoi | Politique de données relue à préparation/lecture applicative HTTP/chargement/octroi/consommation ; tuple restricted/local-attestation/swarm/sealed-attestation autorisé explicitement | Seul helper fixe, aucun fournisseur général |
| Secret limité à action et service | Grant lie identité/session/digest/mandat/révision/destination/contexte ; HMAC natif vérifié, matière absente des exports | Mémoire serveur éphémère ; swap/traçage privilégié hors qualification |
| API/CLI/Admin partagés et persistés | Listes étendues sauvegardées/réouvertes exactement en FR/EN/mobile ; valeurs refusées ferment le flux | Les actions métier restent public/internal, ressources classées séparément |
| Exfiltration, contournement, reprise/refus | Gardes OS réelles du helper, substitution d’image reproduite puis bloquée, retrait de destination/rejeu/reprise fermée | Pas de preuve de confinement général des providers |

290cas Go significatifs sous race en une série,17contrôles publics sous politique
active,16legacy et19navigateur passent. Les noms et entrées sont dans le reçu ;
ces unités ne constituent pas un pourcentage de qualification.

Réexamen du besoin : les critères T4 du plan exigent de contrôler les chemins
réellement pris en charge et de refuser les frontières non démontrées. Ils
n’autorisent ni n’imposent l’activation d’un compte externe ou d’un fournisseur
arbitraire. La cible sensible locale du helper est maintenant positivement
vérifiée ; les autres chemins restent non pris en charge et refusés. Leur
qualification future constitue une extension distincte, pas une acceptation
inventée de T4. La recette globale, la revue applicable et le RETEX restent à
terminer ; R20 noyau positif/surcoût demeure ouvert.

## Historique de l’inventaire avant ce raccord

# R14 — chemins contrôlés et dépendances de la suite

État observé le 15 octobre 2026, 22:33 UTC ; propriétaire Codex. Le reçu courant
est `verification-confinement-20261015/introspection-verification.json` ; il couvre
également les régressions des données, du confinement et du courtier. Ce document
ne remplace ni les preuves originales ni une acceptation indépendante.

| Chemin | Contrôle actuel | Résultat et limite |
| --- | --- | --- |
| Demande, accord et premier effet simulé | trust_data_policy.go ; classification par ressource administrée, contexte lié au digest | Vérifié pour public/internal, local-ledger/swarm/simulated-settlement |
| Refus de données sensibles ou destination inconnue | admission avant original, réponse ou preuve métier | Vérifié ; aucun contenu sensible admis par ce chemin |
| Octroi par action approuvée | trust_secret_broker.go ; session, principal, digest, mandat, révision, destination et usage unique | Vérifié pour sealed-attestation/local-attestation ; contexte d’identité consommé et preuve liée |
| Consommation du helper fixe | trust_secret_consumer.go et trust_sealed_linux_amd64.go | Landlock, seccomp, FD, environnement et réseau contrôlés ; aucune extension arbitraire |
| Matière serveur | mémoire uniquement, non dumpable, expiration/retrait/révision ; reprise fermée | Vérifié dans fixtures ; swap et traçage privilégié hors qualification |
| Révocation des sessions de services | trust_introspection*.go, validation commune sous verrou | Vérifié chez émetteur isolé ; ce connecteur d’identité ne confine pas les providers |
| Fournisseurs IA et effets sensibles externes | capacités non qualifiées, refus de cette admission | Non livrés dans la portée sensible ; AIConnection.Key ne prouve pas un octroi par action |

R14 demeure partiel pour la cible d’entreprise. Rien n’active une destination,
un compte, un paiement réel ou une observation extérieure. La condition de reprise
du chemin sensible est un connecteur borné effectivement confiné et une recette
isolée reliant octroi exact, destination et absence d’exfiltration. Propriétaire
Codex ; aucune décision utilisateur nécessaire pour préparer et vérifier ce contrat.

Les interfaces d’identité et d’admission sont disponibles pour la tranche T5 en
public/internal avec effet simulé. Préparer les définitions versionnées, dossiers,
transitions et exceptions, leur API/CLI et une UX Processus construisible séparément.
Rechercher les producteurs et consommateurs existants avant d’écrire ; le contrat
R15 impose version fixée, refus sans mutation, séparation des responsabilités,
reprise et recette du build distinct. Cela ne clôt pas R14 ni le reste du programme.
