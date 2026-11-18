# Passages de remplacement proposés pour l’article v6

État du 18 novembre 2026 : corrections source, contrôles ciblés et un scénario
natif concurrent vérifié sur fixture ; sans installation. L’article original reste intact
(SHA-256 `25337e620ab427331457c3e6f04df44ea7afe330b8f2b05edc0e59c9c6374271`).
Les passages ci-dessous sont un delta de rédaction à intégrer après décision
éditoriale ; aucune publication ou envoi n’a eu lieu.

## Résultats et limites du prototype

Le candidat Swarm conserve explicitement l’incertitude lorsqu’une erreur
survient après une publication Git possible. Un effet déjà enregistré reste
connu même si une fermeture échoue ; une erreur ne vaut jamais preuve de retour
arrière. Reconcile distingue désormais l’histoire du journal et la valeur de
la référence Git relue : la référence seule ne permet pas d’attribuer son effet
à une intention, et un journal vide ne prouve pas l’absence d’une publication.
Ces contrats sont couverts par des contrôles ciblés et une revue source ; leur
qualification sur le parcours signé natif et leur installation restent distinctes.

L’admission et l’attente contrôlée de la transaction utilisent un bail dérivé
du minimum des expirations signée et du mandat. Son échéance monotone est fixée
une fois, sous l’hypothèse d’une horloge UTC fiable lors de la dérivation. Les
lectures Git imbriquées héritent de l’échéance parente. Ce bail ne garantit pas
l’instant physique de publication après l’envoi de COMMIT à Git ; il ne démontre
donc pas H2 à l’égard de toute expiration au point d’effet.

Un modèle TAMARIN minimal reproduit la fenêtre de la figure 3a sans section
verrouillée. PR est réfutée dans cette version et vérifiée dans la version
verrouillée, avec témoins d’engagement légitime dans les deux cas. Le modèle
porte sur la sérialisation définie, sans expiration, panne, journal ni preuve
de correspondance complète avec l’implémentation. Voir l’[annexe PR](ARTICLE-V6-ANNEXE-PR.md).

L’oracle a été appliqué hors ligne aux 83 rangées conservées d’A0 ; il ne s’agit
pas de 83 nouvelles expériences ni d’une évaluation du binaire Swarm. Le tableau
conserve les cas inconclusifs et les passages vacuitaires. Il détecte le
contre-exemple PT prévu et un échec PV2 lorsque l’identifiant désigne une
intention réutilisée ; la projection par tentative donne un verdict différent.
L’identification de i doit donc être explicitée avant toute revendication PV2.
Les réponses unknown ne déclenchent pas les implications PV1/PV2, ce qui ne
constitue pas une preuve positive de fidélité ou de récupération. Voir l’[annexe A0](ARTICLE-V6-ANNEXE-A0.md).

Un test natif avec barrières observe le retrait pendant la section partagée
réelle de Commit. Après un témoin signé valide, le retrait public rencontre
EAGAIN sur le même inode ; Trust reste inchangé tant que la section est tenue.
Après libération, l’engagement est enregistré, le retrait devient effectif et
un nouvel usage est refusé sans nouvel effet ; Reconcile sépare effet historique
et référence actuelle. Un test nommé passe, et l’intégrité de la recette est
vérifiée. Cette observation H2 est bornée au scénario coopératif exécuté ; elle
ne couvre ni toute expiration au point physique Git ni une médiation universelle.
Le premier essai en échec et les incidents de méthode restent conservés.

## Correspondance affirmation → preuve lue

| Proposition | Source ou observation | Portée exacte |
| --- | --- | --- |
| Incertitude et effet connu conservés | `paea_git_promote_call.rs` : enveloppe Effect, observed_unknown ; contrôles P1 et revue ciblée09 | Composants, pas installation |
| Histoire et ref séparées | Même source : effect::reconcile, read_current_ref ; gardes P2 et revue09 | Lectures Git ciblées, pas attribution par la ref seule |
| Bail fixe et bornes imbriquées | PublicationLease, managed_repository::git_with_deadline ; douze contrôles P3 et revue09 | Admission/attente contrôlée, pas point physique Git |
| Contre-exemple PR puis preuve symbolique | Deux théories conservées, six verdicts TAMARIN ; annexe PR | Modèle défini seulement |
| Oracle effectivement appliqué | Évaluation r4, exit0 + COMPLETE hashes ; annexe A0, source oracle `22ce18d5e9230a2d96399ea4ef14c7a615c99369ecd45af34ed6e458c9db2e66` | Corpus A0, projections explicites, aucune conformité globale |
| Retrait concurrent observé | P4 Q3B-R1 : 1 test natif PASS, recette2steps exit0 et verify0, ELF08b99… | Fixture coopérative : contention, retrait, refus et Reconcile ; pas H2 globale |

L’hypothèse de portage retenue pour le premier contact reste un doctorant recruté
sur EvalIA. Elle désigne une proposition, sans financement ou recrutement acquis.
