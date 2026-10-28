# Replan — méthode et réutilisation

Après remise corrective 430, huit contrôles T2 et sept contrôles T1 passent sur
le candidat intégré 2b0694d693872bf1c0e674060ebd42094e8e58f6. Le précontrôle
431 refuse ensuite la réutilisation d’anciennes inspections ; pause publique432,
84 appels conservés. L’enveloppe d’erreur citait T1 ; la cause est le prédicat
partagé dans planLegacyCandidateFragments pour l’ancien avis T2 R19.

Le refus d’une nouvelle dépense après simple reformulation et l’éligibilité à
réutiliser une inspection sont deux décisions différentes. Le premier doit
rester refusé ; une méthode différente exige des inspections fraîches pour un
candidat effectivement corrigé. Le contre-exemple échoue avant correction ;
les deux cas passent ensemble avec race après comparaison de WorkflowDigest.
Aucune exception de mission, ancienne preuve modifiée, réutilisation ou remise
à zéro des tentatives et appels n’est introduite.

Propriétaire Codex. Prochaine action : livrer ce correctif ciblé puis rejouer
l’opération publique déjà attribuée, avec son même événement et sa demande
originale. Les contrôles ne sont réutilisables que si leurs entrées restent
identiques. L’avis indépendant nouveau et son effet restent à observer.
