# Brouillon — résumé et conclusion

Statut : brouillon du 7 octobre 2026. Chiffres repris de la section 7, eux-mêmes issus des
fichiers de résultats. Contributions revendiquées : C1 (modèle et invariants), C2
(implémentation et correspondance invariant → test), C3 (protocole d'injection et résultats).
**C4 (attribution) n'est pas revendiquée** : l'étiquetage n'est pas fait. C5 est absorbée par
la section 6 (le banc de facturation est l'évaluation elle-même).

---

## Résumé

Les agents fondés sur des grands modèles de langage commencent à préparer des opérations à
effet irréversible, comme le paiement de factures. Dans ce cadre, une revue finale arrive après
l'effet. Nous défendons qu'un moteur déterministe doit médiatiser chaque lancement, chaque
reprise et chaque acceptation, et que les agents doivent seulement proposer. Nous formalisons
huit invariants de médiation et trois propriétés de l'exécutant, et les mettons en œuvre dans
Swarm, un moteur ouvert en Go. Nous l'évaluons sur un banc de facturation simulé, sous huit
types de fautes injectées (réponse perdue, lanceurs concurrents, arrêt brutal, candidat modifié,
service indisponible, budget épuisé, propriétaire expiré, rapport ancien), croisées avec trois
politiques de clé d'idempotence : 9 000 exécutions avec agents scriptés, puis 90 avec un agent
réel. Sur 2 968 exécutions valides, le moteur ne produit aucun paiement inexact ni aucun faux
succès, là où une chaîne sans moteur en déclare 1 900 sur 3 000. La clé d'idempotence reste
nécessaire contre les doublons ; le moteur ajoute la protection contre les candidats modifiés
ou périmés et contre les succès non fondés. Ce gain coûte un facteur 8 à 30 en durée. Avec un
agent réel, un workflow fixe reprenant les mêmes contrôles obtient les mêmes résultats sur les
fautes testées : l'apport propre du moteur porte sur la concurrence et la reprise durable.

*(≈ 220 mots.)*

---

## 11. Conclusion et travaux futurs

**Ce que montre l'évaluation.** Sur un processus de paiement simulé, séparer la proposition de
l'action suffit à supprimer, dans nos conditions, les deux défauts les plus coûteux d'une
chaîne d'agents : payer un candidat modifié ou périmé, et déclarer réussi un règlement qui ne
l'est pas. Le moteur ne remplace pas la clé d'idempotence ; les deux protections couvrent des
fautes différentes et se combinent. Le partage de l'arrêt l'illustre : une modification du lot
après le départ du règlement est arrêtée par l'exécutant, qui vérifie l'empreinte du candidat ;
une modification avant ce départ, par le moteur, qui refuse une preuve qui n'est plus fraîche.
Aucun des deux mécanismes ne couvre seul les deux moments.

**Ce qu'elle ne montre pas.** Avec un agent réel, un workflow fixe qui reprend les règles du
moteur obtient les mêmes résultats sur les fautes testées : ce que Swarm apporte en propre
(baux, lancement exclusif, reprise durable) ne se mesure ici qu'avec des agents scriptés. Le
responsable de mission et la revue restent scriptés. L'évaluation porte sur un seul scénario,
une API simulée et un modèle. Le coût en durée est réel et non optimisé.

**Le moteur comme source de faute.** L'évaluation a mis au jour cinq défauts du moteur et un
choix de sûreté à documenter : blocage en espace de travail propre, refus silencieux,
indisponibilité traitée comme un échec métier, verrou SQLite non réessayé, couverture des
exigences vérifiée à la clôture et non avant l'effet (ce dernier constaté à la lecture du code,
pas en exécution) ; l'absence de reprise automatique après un arrêt brutal relève du choix de
sûreté. Le verrou SQLite est la seule cause des exclusions de la campagne. Un moniteur de référence doit être lui-même évalué ; ces défauts
sont rapportés aux mainteneurs.

**Travaux futurs.**

1. *Attribution des blocages* (QR3) : étiquetage du corpus conservé, comparaison du diagnostic
   du moteur à la vérité par construction, accord humain–modèle.
2. *Coordination par des modèles* : responsable et vérificateur tenus par des agents réels, le
   moteur gardant seul l'autorisation de l'effet ; fautes visant la coordination (injection dans
   une remise, revue favorable sur un contrôle en échec, exigence mal rattachée).
3. *Fautes propres aux modèles, plus réalistes* : la facture piégée testée a été déjouée par le
   modèle lui-même ; il faut des fraudes plausibles pour mesurer ce qu'apporte le contrôle.
4. *Revues décorrélées* : modèles différents pour la préparation et la revue.
5. *Systèmes réels* : API de paiement réelles, conservation limitée des clés, rapprochement
   bancaire.

---

## Points à vérifier

1. « Huit types de fautes » : F1 à F8 ; F4e est une variante de F4 (candidat modifié avant le
   départ du règlement) et F9 (facture piégée) n'existe que dans le lot réel. Ajuster si la
   section 6 présente F4e comme une faute distincte.
2. Le résumé dépasse 200 mots ; à réduire selon la limite du lieu de soumission.
3. « Les deux défauts les plus coûteux » est une appréciation : la justifier en section 2 ou la
   retirer.
