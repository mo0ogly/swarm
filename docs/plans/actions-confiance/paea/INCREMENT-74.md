# PAEA74 — protocole de transfert Git sans référence, preuve préalable

Reprise28oct11:47UTC : dernier73, version230/PID3682612/ELF
0b8cf6947596da380ed2f9ee862ed80597f949b2d3607d2147a787fd0d051287 observés.
Aucun Cargo actif,118Go libres. Codex direct sansGLM/délégation/commit/push.

Replan PA03 : fetch par chemin /proc/PARENT/fd pose une dépendance à conserver
après erreur d'enfant non récolté ; capturer un pack arbitraire en Vec n'est pas
une réponse sûre. Comparaison : fetch existant avec source détenue dans chaque
issue, ou export pack dans un fichier privé puis import par stdin propre à
l'enfant, cwd ancré et aucune refspec. Avant code de service, qualifier les
commandes exactes pack-objects --revs --stdout / index-pack --stdin --fix-thin.
La preuve74 est une recette de protocole, pas un transfert natif implémenté,
une autorisation courante ou une installation du service protégé.

Critères : vrai candidat cumulatif71 relu via CLI230 normale ; export streaming
sur disque, import/présence du commit et aucune référence modifiée ; idempotence
de transfert ; SHA1/SHA256 incompatible, pack tronqué/corrompu et OID absent
refusent avec originaux conservés ; reprise valide. Pas de modèle payant, root
utilisateur utilisé en fixture, boucle de retry ou deadline arbitraire.
Les objets/files éventuellement écrits après refus sont inventoriés, jamais
rebaptisés "aucun effet". Borne expérimentale : petite fixture native dont la
naissance est retenue ; aucune conclusion de budget/volume du service général.
PA02/03partiels,0/17/humain/UID/confinement/daemon/E2/E3/PA13/RETEX ouverts.


## Résultat observé

Preuve : `/home/fpizzi/Documents/ChatGPT/swarm/docs/plans/actions-confiance/paea/evidence/20261028-pack74-0s4oq4jo`. Protocole01 : six assertions nommées PASS/VERIFIED.
Git réel2.51.0 SHA2563446b26047255f16d588eda4527aeedf02bd173449382d93bfe8a141257876c9.
Pack2199octets SHA256dad020a833d330307f1936342b8281fba938cf74d4b90b08377ef0f621c103d5.
Candidat23a4b21b66d62b8928b5a979e3d6dedd62df6653, Work source identique avant/après.
Le refus de format incompatible se manifeste ici par un échec inflate, pas par
un diagnostic natif dédié : conserver le précontrôle de format73 avant import.
Aucun transfert natif ni autorité métier ajouté. La borne expérimentale de volume
est2199octets ; aucune extrapolation de ressources/latence du service.

Revue des octets : fonctions publiques/non triviales du script documentées (but,
préconditions, canaux, effets partiels et limites). Valeurs65536 (bloc de lecture),
format SHA1/SHA256, refs de fixture, OID absent et altération offset20 sont des
entrées/protocoles techniques de test, pas politiques opérateur. Aucun timeout,
retry ou plafond produit ajouté. Les retours Git sont vérifiés, stderr original
retenu ; inventaire après import échoué, aucune conversion refus→succès vide.
Pas de source/workflow produit modifié : documentation FR/EN et recette suffisent,
pas de Cargo, notes/version nouvelle ou redémarrage requis pour ce lot.
Une faute de chemin documentaire lors de lecture a été corrigée par inventaire
rg avant édition ; aucune commande dépendante ni fichier produit modifié.
