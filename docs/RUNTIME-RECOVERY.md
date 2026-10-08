# Incident machine et reprise d’une mission

Une mission n’est terminée que lorsque ses résultats sont validés et son
responsable a clôturé le travail. Un serveur actif ou des tests du moteur réussis
ne prouvent pas cette clôture.

## Stockage indisponible

Le cockpit affiche un bandeau global et un bouton **Diagnostic du stockage**.
Cette lecture locale ne consomme aucun appel IA et ne dépend pas de SQLite :
elle reste accessible si la lecture des missions échoue. **Vérifier à nouveau**
actualise les mesures ; aucun bouton ne supprime de fichier.

En ligne de commande :

```sh
swarm --root /chemin/du/projet doctor
swarm --root /chemin/du/projet --json doctor
```

`doctor` ne crée ni n’ouvre la base. Il lit les volumes de `.swarm` et du dossier
temporaire du processus. Le code de sortie vaut 2 si l’un des volumes est
indisponible ou sous la marge de sécurité, 0 sinon. Le JSON donne les chemins,
octets et inodes disponibles. Un dossier `.swarm` absent n’est pas considéré sain.

- Avertissement sous 1 Gio disponibles.
- Nouveaux départs suspendus sous 256 Mio, ou sous 256 inodes disponibles.
- Une mesure impossible suspend également les départs.
- Les préparations d’agents, départs de superviseurs, activations de planification
  et revues indépendantes utilisent le même contrôle avant un nouvel appel.
- Le conducteur réexamine le stockage à chaque cycle. Après rétablissement, il
  reprend la réconciliation et les opérations déjà autorisées. Les tâches refusées,
  plafonds épuisés et pauses restent applicables.
- Les erreurs système ENOSPC et SQLite FULL deviennent `storage_unavailable` ;
  l’API renvoie HTTP 507, sans annoncer que la commande a réussi.

Le contrôle n’est pas une réservation d’espace. Un agent déjà actif peut remplir
le volume entre deux lectures. Les volumes propres à un fournisseur distant ou à
un espace externe ne sont pas couverts par cette mesure. Ne pas interpréter
« stockage disponible » comme une garantie de capacité pour toute la mission.

Pour libérer de l’espace, identifier d’abord ce qui l’occupe. Les caches de
compilation reconstruisibles peuvent être nettoyés par leurs outils. Les rapports,
bases, copies de travail en échec et historiques ne sont jamais assimilés à des
caches. Après une écriture interrompue, examiner l’état public avant toute reprise.

## Tentatives épuisées

Le cockpit affiche **Mission bloquée — tentatives épuisées** lorsqu’aucune tâche
ne tourne et qu’une tâche bloquée a atteint son plafond. Le bouton **Examiner les
tentatives et les refus** ouvre les faits conservés, le dernier avis indépendant
et les accès aux rapports. **Enregistrer le diagnostic** exporte un JSON local
avec révision, tâche, tentatives et avis. Aucun appel IA n’est lancé par ce parcours.

Le CLI expose les mêmes faits via `swarm mission status TRAVAIL`, ou en JSON avec
`attempts_used`, `attempts_allowed` et `attempt_limit_reached` pour chaque tâche.

Après trois tentatives, **Préparer un essai correctif** ouvre la proposition
fondée sur le dernier refus indépendant. Une confirmation explicite autorise un
seul essai supplémentaire, avec historique et revue conservés. Les conditions,
limites et commande CLI sont décrites dans [la reprise bornée](ATTEMPT-RECOVERY.md).
Si l’action est indisponible, son motif reste affiché : une autorisation antérieure,
une revue absente ou en cours, ou un budget de revue épuisé ne sont pas contournés.

Cette possibilité de reprise ne démontre pas, à elle seule, qu’une mission réelle
aboutira. La clôture dépend de la correction effective et de ses preuves.
