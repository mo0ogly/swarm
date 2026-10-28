# Reprise après66 — noyau de référence installé et vérifié

Codex direct, sans GLM/délégation/commit/push. Objectif PA00…13/RETEX actif.
Preuves : `docs/plans/actions-confiance/paea/evidence/20261027-settlement66-jxj0c9b6`.

## Résultat local vérifié — 27 octobre 2026

La capsule13 a terminé PASS : témoin valide, 12 tests distincts et Rustdoc strict.
Son intégrité est VERIFIED. core12 reste FAIL pour changement concurrent, red11
reste le contre-exemple du contrôle de séparation absent avant correction.
La capsule conserve 779 entrées, sans liens ; les 22 fichiers de scaffolding des
packages CLI/Web/Biba/API ne sont pas compilés dans cette recette. Leurs packages
sont absents des artefacts observés. Les instructions embarquées n'ont pas changé.

Le build normal14 qualifié possède 677 entrées. Sur les 477 entrées communes avec
la capsule, seule version_history.json diffère (note66 ajoutée après gel). Les
bibliothèques métier ont features vides/test=false, aucun feature fixture. Trois
écarts transitifs time/macros, smallvec/const_generics+const_new, once_cell/default
sont examinés dans feature-review14.json ; pas d'identité entre ELF test et produit
revendiquée. Le test ELF13 est conservé séparément avec son hash et ses artefacts.

Le candidat source **v0.1.1-dev.220** a été activé par le chemin pris en charge
avec --prepared-build et son SHA complet. PID observé2848776, ELF
`90019b2d94ddeaef6e3422257ae2962e21994e9ca4d964feec53b7e93da4dd09`.
56 contrôles Version/UX et garde de script PASS, intégrité VERIFIED ; les25assets
protégés et les notes FR/EN sont comparés à l'inventaire compilé, captures FRclair/
ENsombre inspectées. Ces pages existantes ne consomment pas le nouveau backend.
Aucun paiement réel, nouveau daemon, service protégé ou parcours UX ajouté.

La revue locale source-review.json couvre les octets propres, politiques,
fonctions et erreurs. Les fichiers partagés contiennent aussi des travaux d'autrui ;
leur inventaire ne vaut pas revue exhaustive de ces changements. Les6pages types
Rustdoc et RootStore sont retenues avec hashes. La recette permanente et sa variante
figée ont les mêmes étapes, à l'exception de l'identifiant et de la cible de cache.

Méthode : deux erreurs de lecture/lancement sans mutation (README supposé sous
recipes et option --evidence au lieu de --output pour run) ont été corrigées par
inspection des chemins/aide réels. Aucun test échoué n'a été réécrit. Les preuves
historiques, bases et processus étrangers sont préservés ; aucun commit/push.

Suite : admission attribuée réellement vérifiée, puis service Git distinct et
transport/profil séparé selon PA03-SERVICE-INTEGRATION. Hypothèse de validation de
métadonnées de reçu conservée avant toute exposition publique. PA02/03 partiels,
0/17 invariants globaux, UID/humain/confinement/daemon/E2/E3/PA13/RETEX ouverts.

Revérifier heure, état actuel et changements concurrents avant reprise. Ne pas
relancer13/14/16 : terminal PASS. Helpers UID/admin déjà demandés : ne pas répéter
ni contourner. Heartbeat existant borné28oct19h20Paris, ne pas dupliquer.
Disque2,5Go libres observés, ne pas supprimer preuves ni caches partagés.
