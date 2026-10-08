# Audit de la spécification

Date : 5 octobre 2026. Portée : cohérence du dossier de conception. **Auto-relecture documentaire**, sans revue indépendante du produit, sans tests fonctionnels du futur système et sans acceptation de mission.

## Conclusion

Le dossier permet de commencer la cartographie A01. Les lots de réalisation restent conditionnés aux décisions A02/A03 et à l’examen A04. Les exemples de commandes et de JSON sont des propositions ; ils ne décrivent pas une fonctionnalité déjà disponible.

## Points vérifiés

- 16 lots identifiés et ordonnés ; aucune dépendance vers un lot inexistant et aucun cycle dans l’enchaînement proposé.
- R01 à R18 ont un scénario associé. Les scénarios complémentaires couvrent annulation, autorisation retirée et cible clôturée.
- Même service moteur pour web et CLI ; aucun geste graphique ni déclencheur ne contourne les droits, budgets ou réservations.
- État de présentation distinct du plan métier ; conservation d’un résultat de contrôle distincte de son admissibilité actuelle.
- Rôle de vérificateur distinct d’un résultat accepté ; revue indépendante sur le candidat réellement testé.
- Demande, occurrence, mission et tentative distinctes ; effet incertain conservé et à réconcilier.
- Une erreur de recette ou d’environnement ne justifie pas automatiquement une nouvelle production.
- Documentation FR/EN, quatre variantes de rendu, installation et retour arrière inclus dans les livrables.
- Références techniques isolées dans une annexe ; commentaires et libellés centrés sur le comportement Swarm.

## Corrections apportées pendant la relecture

| Risque | Correction dans le dossier |
| --- | --- |
| Références d’inspiration dans les commentaires ou textes utilisateurs | Dossier nommé par sa fonction ; références regroupées dans SOURCES.md |
| Programmation d’une mission terminée ambiguë | Refus explicite sans clone ni nouvelle tentative ; scénario T27 |
| Permission devenue invalide après prévisualisation | Revalidation à l’application et à la prise en charge ; scénario T26 |
| Pause confondue avec arrêt ou annulation d’un effet | Portées distinctes et scénario T25 |
| Rejouer une suite entière sur changement administratif | Contrat d’empreintes pertinentes et scénarios T15/T16 |
| Commandes proposées prises pour commandes livrées | Avertissement explicite dans contrats et plan |
| Charge d’essai prise pour capacité garantie | Mesure de référence avant adoption d’un budget de performance |
| Confusion avec un autre livrable Office | Mention retirée de la recette |

## Décisions à fermer avant les lots concernés

| ID | Décision et preuve nécessaire | Responsable / étape | Bloque |
| --- | --- | --- | --- |
| DEC01 | Schéma public, opérations réellement existantes et compatibilité des données : inventaire et parcours isolé | Moteur/données, A01 | A02 |
| DEC02 | DTO, codes HTTP/CLI, champs modifiables et empreintes des preuves : ADR et cas de conflit | Moteur/données, A02 | B01/B02 |
| DEC03 | Maintien SVG ou composant indépendant : prototype, licences, bundle, CSP, clavier et quatre rendus | Web/CLI, A03 | B03 |
| DEC04 | Format de récurrence, ambiguïtés de fuseau et durée de conservation des occurrences : contrat et horloge injectable | Moteur/données, A02 puis C02 | Activation de programmation |
| DEC05 | Authentification, stockage des secrets, rotation et limites d’entrée : mécanisme existant vérifié et tests négatifs | Moteur/données, A02 puis C03 | Événements externes |
| DEC06 | Budgets de performance et limites de taille : mesures de référence identifiées, paramètres visibles | Web/CLI et moteur, A03 | Adoption du renderer et imports |
| DEC07 | Contrôles ciblés exacts et capacités hôte/fournisseur : matrice exécutable avant revue | Validation/livraison, A04 puis chaque lot | Acceptation des lots |
| DEC08 | Sauvegarde, compatibilité du schéma et occurrences après restauration : recette sur copie | Validation/livraison, D02 | Livraison |

Les choix de routine se résolvent dans les lots responsables. Un élargissement de portée, de droits ou de budgets doit rester explicitement autorisé ; il ne peut découler d’une valeur par défaut.

## Limites de cette relecture

Les contrôles documentaires vérifient identifiants, liens, dépendances et cohérence des propositions. Ils ne démontrent ni comportement navigateur, ni performances, ni autonomie réelle d’un fournisseur. La cartographie décrit des fichiers lus ; A01 doit encore établir la référence comportementale. L’annexe de provenance ne constitue pas une garantie juridique sur une future incorporation de code.
