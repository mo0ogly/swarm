# Reprise BLP-04 — provenance du prompt, 31 octobre 2026

## Besoin, décision et portée

Reprise du plan BLP après l’inventaire BF-05 → BF-07 → BF-16.
Responsable : Codex, développement direct autorisé, sans délégation ni appel payant.
Observation : `PreparationSend` ne transporte pas de principal BLP authentifié ;
un label ou un identifiant ajouté par le client ne prouverait aucune attribution.
Deux voies comparées : accepter des labels dans le DTO (écartée, autorité inventée),
ou conserver d’abord les octets du producteur et leurs origines (retenue, additive,
sans activer une protection incomplète). Les attributions persistées et le principal
courant restent nécessaires avant l’admission de transport.

Réalisé : assembleur avec couverture des octets, registre privé v3 obligatoire,
validation à la lecture native avant dispatch/worker/rejeu, refus sans mutation
pour une incohérence. Couverture : contexte projet agrégé, instructions moteur,
contrat de plan et neuf champs JSON. Aucune étiquette de confidentialité n’est
attribuée ; les sous-fichiers d’une source agrégée ne sont pas individuellement
résolus. BLP-03 migration, BLP-04 attribution et héritage, BLP-05 destination et
médiation et BLP-06 déclassification restent ouverts. Les acquis Biba restent requis.

## Preuves et incident de revue

- `verification-blp/20261031-prompt-provenance-01/` : neuf tests passent, mais ce
  premier contrôle ne vérifiait pas la correspondance nom d’origine/clé JSON.
- Contre-exemple conservé dans `verification-blp/20261031-prompt-provenance-02-counterexample/` :
  permutation de deux noms d’origine sans changer les octets ni leurs empreintes.
  Ce cas doit refuser ; son résultat et la correction restent à vérifier.
- La recette publique CLI est actualisée pour v3 et compare le digest produit
  au stdin effectivement reçu par un programme inerte. Exécution à qualifier.
- État local observé avant livraison : `./swarm.sh status` indique `running:false`,
  URL configurée `127.0.0.1:38433`. Aucun serveur n’est déclaré installé par ce suivi.

## Revue des contrats (à lier aux empreintes finales)

Politiques : schémas 1/3, noms d’origines et neuf champs sont des invariants de
représentation ; aucune durée, tentative, budget ou plafond opérateur nouveau.
Les limites existantes de contexte continuent à porter sur les octets réels du
prompt. Le texte d’instructions préexistant est conservé à l’identique.

Fonctions non triviales : `assemble`, `Manifest::verify`, `append`, `hash`,
`native_metadata_valid`, `decode_present_provenance`, producteur `preparation_send`
et consommateur `record`, helpers de test et `private_shape`. Contrats documentés
avec absence d’authentification et d’autorité ; la provenance ne confine pas un
écrivain du stockage. Les DTO publics n’exportent pas le registre.

Erreurs : échec de schéma/encodage à l’assemblage avant sauvegarde ; échec de
provenance à la lecture native avant passage au worker et avant mutation de son
état/budget. Les erreurs SQL/codec/finalisation existantes restent conservées.
Ni champ absent dans v3 ni null ne donnent une réussite vide ou un repli historique.
Un ancien format reste explicitement sans provenance, sans garantie BLP ajoutée.

Prochaine condition observable : classification persistée de toutes les entrées
et principal authentifié, puis témoin d’envoi autorisé et refus sans octet reçu,
révocation, changement de source et héritage de la réponse. Ce registre ne clôt
aucun de ces critères. Aucune acceptation indépendante n’est affirmée.
