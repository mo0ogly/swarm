# Clôture — lancement clair et reprise fiable

Le 3 octobre 2026, à la révision 332, les six tâches T0–T5 sont acceptées. Le moteur confirme 6/6 résultats validés, six gates courantes valides, aucun agent actif et aucun agent incertain. La revue indépendante 45 a rendu PASS sur les trois critères de T5. L’acceptation a été confirmée dans l’interface normale, sans dérogation ni modification directe de SQLite.

Le candidat de 48 fichiers reste a0b89b5c7f8ba687ddb729ed6fff82b051456004f202b59b9891cbceddbc7e77, basé sur 1d9570bd4ef617130c6be96b7ec88844fdbcd00e avec modifications locales. Suite Go complète PASS (473,034 secondes), vet, race ciblée et frontend PASS. Les cinq contrôles finaux T5 ont un code de sortie 0 ; les captures et séquences clavier ont été examinées par la revue indépendante. Preuve de clôture : CLOTURE-r332.json.

## Ce qui a débloqué la clôture

- Réduire le rapport et dater l’instantané des consommations ; distinguer historique figé et compteur vivant.
- Retirer la proposition de livraison prématurée : revue actuelle, contrôles frais et acceptation restent des étapes distinctes.
- Fournir le contenu des séquences clavier au vérificateur : une référence empreintée n’est pas un contenu accessible. QW3 a été rejoué dans les quatre variantes, avec captures du focus et retour au déclencheur.
- Employer la reprise publique explicite après autorisation du plafond 45. Le moteur conservait un message de budget 44 épuisé ; changer le plafond seul n’avait pas effacé ce diagnostic. L’historique et les appels consommés sont conservés.

## Limites et suites réellement restantes

Cette clôture concerne les quatre quickwins et leur recette supervisée. Elle ne prouve pas une autonomie sans intervention humaine et ne certifie pas tout le produit. Les revues 41/42 ont demandé des corrections, 43 a expiré, 44 a demandé les preuves clavier, 45 a abouti. Ces appels ne sont pas effacés.

L’interface affiche bien 6/6, mais conserve aussi un ancien diagnostic du responsable et des retours historiques. Une amélioration de présentation reste nécessaire : distinguer les incidents historiques de l’état des résultats clôturés. De même, expliquer explicitement la reprise requise après changement de plafond, plutôt que conserver un blocage de budget ambigu. Ces améliorations ne sont pas annoncées implémentées par cette annexe.

Les fichiers soumis aux gates et revues restent inchangés ; cette annexe est une preuve ultérieure de clôture. Aucun commit, push ni merge n’est réalisé dans cette étape.

## Enseignements complémentaires — 4 octobre 2026

Cette synthèse complète le bilan sans modifier les rapports validés. Les faits détaillés sont consignés dans [le journal de supervision](RETEX-supervision.md) et [le RETEX de la revue 43](RETEX-review43-timeout.md). Les actions proposées ci-dessous restent à instruire ; leur inscription ne vaut pas implémentation.

| Enseignement | Fait observé | Amélioration proposée |
| --- | --- | --- |
| Une consigne ambiguë peut empêcher la réalisation. | T1 a interprété « deux corrections » comme une restriction empêchant un changement cohérent de quatre fichiers ; la tentative a livré un diagnostic sans correctif. | Séparer résultat attendu, fichiers autorisés, cycles de correction et budget d’outils ; vérifier les contradictions avant le départ. |
| Une référence à une preuve ne suffit pas à la rendre accessible. | Le vérificateur sans outils ne pouvait pas ouvrir les fichiers cités. La revue 44 a également demandé le contenu des séquences clavier. | Construire un dossier contenant les preuves nécessaires, leur provenance et leurs limites ; vérifier son accessibilité dans le contexte réel du vérificateur. |
| La taille du dossier de revue doit être maîtrisée. | La revue 43, avec vingt captures, a expiré après 300 secondes sans verdict final. Cela ne démontre pas que la taille était la seule cause. | Expérimenter des revues par périmètre ou critère, puis une synthèse liée au même candidat ; mesurer couverture, coût et durée avant de généraliser. |
| Un critère de clôture peut devenir circulaire. | T5 mélangeait l’évaluation du dossier et l’obtention du futur avis favorable de cette même revue. | Faire évaluer les livrables et preuves par la revue, puis faire contrôler avis courant, fraîcheur et acceptation par le moteur. |
| Les unités des compteurs doivent rester distinctes. | Une remise manuelle ajoutait un enregistrement de tentative sans nouvelle exécution ; les signaux thinking_tokens n’étaient ni des appels d’outils ni un total de jetons. | Afficher séparément processus, remises, outils, revues et événements fournisseur ; conserver « inconnu » pour les mesures absentes. |
| Une recette supervisée ne démontre pas une autonomie sans aide. | Le superviseur a implémenté une partie des corrections, composé les dossiers et réalisé des interactions de recette. | Attribuer chaque intervention ; mesurer le nombre et le temps des interventions humaines ou externes nécessaires à la clôture. |

### Mesurer l’effort pour obtenir un résultat validé

Le résultat final doit être accompagné d’un bilan par tâche et par tentative : durée, outils observés, revues, reprises, coût et jetons rapportés, interventions externes. Chaque chiffre doit porter une date, une révision et une source. Un coût inconnu ne vaut pas zéro ; un événement d’activité ne vaut pas une requête payante.

Pour analyser les reprises, distinguer : travail du producteur, préparation et accessibilité des preuves, comportement du vérificateur, règles et présentation du moteur, contraintes d’environnement. Plusieurs causes peuvent contribuer au même incident ; les traces disponibles ne permettent pas toujours de les départager. Ne pas attribuer l’ensemble des appels aux agents ou au moteur sans cette analyse.

### Suivi restant

- [ ] Consolider le bilan des consommations et interventions par tâche, en signalant les données manquantes.
- [ ] Prioriser les améliorations de consignes, dossiers de revue, compteurs et déblocage à partir de ce bilan.
- [ ] Comparer une revue globale et des revues ciblées sur un périmètre équivalent, avec les mêmes exigences de preuve.
- [ ] Intégrer ces enseignements dans l’article « théorie → pratique », en distinguant résultats démontrés et hypothèses.

Le lot a ensuite été commité et poussé sur `codex/clear-launch-recovery` : [cc3069d](https://github.com/mo0ogly/swarm/commit/cc3069dc7bb61b90168d21f945cb2eb5e27578ed). Cette publication est postérieure à la clôture r332 ; elle ne constitue pas une nouvelle recette ni une fusion dans la branche principale.

## Quick wins proposés — 4 octobre 2026

Ces quatre actions sont ajoutées au suivi, pas annoncées réalisées. Elles visent des corrections ciblées sans refonte de l’interface. Ordre proposé : QW5, QW6, QW8, puis QW7. Les identifiants prolongent les quatre quick wins du lot clôturé.

| ID / priorité | Action | Critère de vérification |
| --- | --- | --- |
| QW5 / 1 | Depuis un blocage de budget, bouton « Régler cette limite » ouvrant le réglage exact de la mission, avec consommation, plafond, restant et effet de la reprise. | Le bouton cible la bonne mission et la bonne limite. Aucun plafond ne change sans confirmation explicite. L’historique est conservé ; la reprise nécessaire est expliquée. La CLI indique la commande et la portée correspondantes. |
| QW6 / 1 | Séparer état actuel et incidents historiques ; ranger les incidents résolus dans un historique consultable. | Une mission clôturée affiche son état courant sans ancien blocage actif ; un blocage réellement courant reste visible. Les événements historiques ne sont ni supprimés ni réécrits. Même distinction dans la CLI. |
| QW7 / 2 | Résumé d’effort par tâche : durée, outils observés, revues, reprises et coût rapporté. | Les chiffres concordent avec les traces, portent une date ou révision et distinguent les tentatives. Les mesures absentes affichent « non rapportées » ; aucun coût nul ni total de jetons n’est inventé. Résumé accessible en web et CLI. |
| QW8 / 1 | Contrôle déterministe du dossier avant revue : preuves requises présentes, accessibles au vérificateur et courantes selon le contrat. | Un dossier incomplet ou périmé donne un motif et une action avant tout appel fournisseur. Un dossier admissible peut poursuivre la revue indépendante ; le contrôle préalable ne produit aucun verdict de qualité ni acceptation. Même règle moteur pour web et CLI. |

- [ ] QW5 — accès au réglage de la limite concernée.
- [ ] QW6 — présentation de l’état courant et de l’historique.
- [ ] QW7 — résumé d’effort avec provenance et mesures inconnues.
- [ ] QW8 — contrôle préalable du dossier de revue.

Recette attendue pour chaque action : parcours normal, erreur et reprise, FR/EN, deux thèmes, clavier et focus pour le web ; parcours CLI équivalent sur racine isolée. Ne pas relancer une mission clôturée pour servir de fixture de test.
