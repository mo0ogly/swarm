# Itération 2 : comparer la décision avec et sans mémoire

Reprise du 16 octobre 2026 après « next ». Responsable : Codex seul. Le code du
premier lot mémoire est inchangé depuis sa recette ; les autres changements du
checkout sont préservés. Méthodes : reprise `replan`, périmètre borné APEX.

## Question et protocole

La mémoire améliore-t-elle une décision de reprise, sans appliquer aveuglément
une ancienne recette à une variante ? Cible : C05/C10/C11/C14 du cadrage.
Le premier essai évalue une proposition d’action ; il ne suffit pas à prouver
l’exécution autonome, l’apprentissage général ou un gain statistique.

Le scénario part du refus réellement rencontré dans l’itération 1 : importer
une archive n’installe pas sa preuve dans la racine cible. Une décision historique
ne dispense pas d’évaluer les fichiers actuels. Deux situations synthétiques,
fondées sur ce contrat, forment quatre entrées :

| Situation | Sans rappel | Avec la procédure versionnée | Décision attendue |
| --- | --- | --- | --- |
| Preuve cible absente, original d’archive disponible et empreinte conforme | 1 réponse | 1 réponse | Vérifier l’archive, restaurer l’artefact puis évaluer la cible ; admission seulement après contrôle frais réussi. |
| Preuve cible différente, original vérifiable indisponible | 1 réponse | 1 réponse | Bloquer l’admission et demander une preuve fraîche ; ne pas recopier une ancienne acceptation. |

La procédure est admise par les opérations mémoire normales depuis un RETEX de
fixture vérifié. Les contextes proviennent du véritable `prepareLaunch` en aperçu.
Les originaux complets sont conservés dans les fichiers `.engine.txt`. La sélection
incrémente légitimement la révision du travail : ce seul numéro d’identité est
neutralisé dans les entrées `.txt`, avec transformation vérifiée et déclarée.
Retirer alors exactement le bloc mémoire et son attribution restitue l’entrée
sans rappel octet pour octet. Les fichiers neutralisés sont des entrées d’étude,
pas une affirmation de leur envoi à un fournisseur. Aucun fournisseur ni agent
n’est lancé pour les fabriquer. Les acceptations sources sont celles d’une
fixture de test, sans valeur de recette métier.

## Contrats et critères

| ID | Travail et propriétaire | Vérification observable |
| --- | --- | --- |
| M6.1 Codex : quatre contextes comparables | Exports complets, empreintes, manifeste, source/version et conditions ; aperçus sans tentative ni mutation. |
| M6.2 Codex : grille explicite | Décision, ordre des actions, admission conditionnelle et acceptation inchangée ; rejet des réponses manquantes, dupliquées ou invalides et d’un contexte altéré. |
| M6.3 Codex : contrôles négatifs | La bonne réponse de la situation initiale doit échouer sur la variante. Un vieux succès, une acceptation directe ou un champ de résultat manquant ne peuvent pas produire PASS. |
| M6.4 Utilisateur : essai réel borné | Fournisseur, modèle, niveau, nombre d’appels et budget explicites ; même configuration, conversations distinctes sans outils et sans connaissance de la réponse opposée. |
| M6.5 Codex : résultat et RETEX | Originaux, erreurs, contenu reçu, décision, temps/consommation disponibles et limites. Zéro gain observé reste un résultat possible. |

Comparer des réponses libres sans contrat donne une appréciation difficile à
reproduire. Employer un second LLM comme juge ajoute des appels et une autre
incertitude. Choix de cette étape : réponses JSON et grille déterministe sur une
décision restreinte, avec explication libre conservée mais non notée lexicalement.
Ce choix ne mesure pas la qualité générale de l’explication.

La recette ne modifie pas le produit, les politiques Admin ou les bases vivantes.
Les fournisseurs locaux existent, mais leur présence n’autorise pas une campagne
payante. Préparer et vérifier les entrées et la grille avant de demander uniquement
le choix/accord manquant pour quatre appels réels ; poursuivre les travaux locaux.

## Accord et exécution du 16 octobre

L’utilisateur répond « ok » à la proposition explicite : quatre appels réels
maximum avec le Codex configuré `gpt-5.6-sol`, effort `medium`, sans relance
automatique. Cet accord couvre M6.4 et son bilan M6.5, sans mission Swarm, outils,
modification du produit ou appel supplémentaire.

L’adaptateur de recette reprend les désactivations de `assistantProvider` et
ajoute une vérification locale du transport : serveur simulé HTTP 503, une seule
requête, aucun outil déclaré, modèle et effort contrôlés. Les réponses réelles
utilisent des processus éphémères distincts et des répertoires temporaires vides.
Les refus et fichiers existants ne sont jamais relancés ou écrasés.
