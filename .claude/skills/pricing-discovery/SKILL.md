---
name: pricing-discovery
description: "Découvrir ou actualiser des tarifs de modèles à partir de sources officielles et préparer leur saisie versionnée dans Swarm, sans prix implicite ni changement de budget."
---

# Découverte sourcée des tarifs

Lire le contrat tarifaire réel du projet avant de produire un import. Dans Swarm,
consulter `internal/engine/pricing.go`, `web/pricing.js` et `docs/BUDGETS.md` :
le catalogue projet est distinct des réservations et des coûts rapportés. Une
recherche ne prouve ni le prix du compte client ni la dépense d'une tentative.

Identifier les fournisseurs, modèles exacts et canaux de facturation concernés
depuis la demande ou la configuration publique existante. Ne pas lire les secrets.
Rechercher et ouvrir les pages officielles actuelles ; conserver URL, date de
consultation, date d'effet si publiée et extrait court permettant de vérifier les
montants/unités. Ne pas utiliser les prix mémorisés ou les seuls extraits de recherche.
Une page inaccessible reste une preuve manquante, jamais un prix nul.

Qualifier chaque tarif : devise, unité, modèle/version, fournisseur facturant,
plateforme/région, type de service (standard/batch/prioritaire), palier de contexte,
cache lu/écrit et durée, remises privées et catégories non couvertes. Ne pas
transférer le tarif direct à un revendeur. Ne pas appliquer un multiplicateur à
tous les modèles sans preuve de sa portée. Ne pas confondre abonnement et API.
Si plusieurs conditions donnent des prix différents, garder les variantes séparées.

Produire une proposition lisible avec chaque montant relié à sa source et ses
conditions, et une comparaison avec les versions existantes. Distinguer observé,
normalisé, inconnu, non applicable et proposition. La conversion par million de
jetons est une conversion d'unité explicite ; aucune conversion de devise ou
hypothèse de volume ne doit être implicite. Montants finis non négatifs ; vide
signifie inconnu, zéro seulement si la gratuité est explicitement établie.

Dans le schéma Swarm actuel, `source` est du texte : y conserver URL, date de
consultation et conditions d'application. `reference_date` indique la date de
référence choisie explicitement, pas une date d'effet inventée. Les quatre prix
sont entrée **hors cache**, sortie, cache lu et cache écrit. Si un tarif à paliers,
une taxe, une catégorie ou un abonnement ne peut être représenté fidèlement,
indiquer la limite et ne pas proposer une moyenne trompeuse à importer.

Une recherche prépare une proposition ; elle n'autorise pas son application.
Si l'utilisateur a autorisé la saisie précise, appliquer via l'Admin ou les
opérations publiques réellement disponibles : `pricing list`, puis `pricing save`
avec `expected_digest` fraîchement lu. Sinon, conserver la proposition et demander
uniquement la décision d'application manquante. Ne pas modifier directement le
fichier de catalogue, remplacer l'historique ou augmenter une enveloppe de budget.
Après conflit, comparer les changements avant de reproposer ; ne pas rejouer à
l'aveugle. Une erreur d'écriture ou de fermeture exige une relecture de l'état.

Vérifier après application par relecture et une simulation à volumes explicités :
version/source/conditions conservées, ancien devis inchangé, coût incomplet encore
inconnu. Une estimation n'est ni une réservation, ni un coût rapporté, ni une
facture. Ne pas lancer de modèle payant pour découvrir ou vérifier son prix.

La fraîcheur désirée, l'actualisation automatique et les seuils de variation sont
des politiques opérateur à persister et exposer dans Admin si elles sont ajoutées ;
ne pas les inventer dans la skill. Documenter l'état réel et la prochaine action.
Cette skill est disponible aux prochaines sessions qui découvrent le dépôt ; elle
ne met pas à jour les contextes déjà démarrés et ne crée aucune tâche récurrente.
