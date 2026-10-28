# Auto-revue locale du canal Go

Candidat : huit fichiers `agent_identity*.go` après `runtime-after-run.json`,
HEAD `4e58f7cbf8200a58f69f1f4d03d657c3313fb642` avec changements locaux. Revue
dans le contexte producteur : aucune indépendance ni acceptation globale.

Verdict local : **request changes**, retour au rôle producteur.

- Fraîcheur : `evaluateAgentIdentityMessage` échantillonne l'horloge sous Trust
  avant de prendre le verrou du registre runtime. Une attente peut dépasser
  l'expiration d'un octroi, alors que l'évaluateur reçoit encore l'ancien temps.
  Argument de chemin confirmé par lecture ; cas discriminant à exécuter avant
  correction : bloquer le registre, observer le verrou Trust acquis, laisser
  expirer un octroi, libérer puis exiger le refus.
- Portabilité UAPI : la recherche initiale a lu les constantes communes sans
  les fichiers générés par architecture. `unix.SO_PASSPIDFD` existe déjà ;
  `NS_GET_NSTYPE` vaut `0xb703` ou `0x2000b703` selon l'architecture. Les ioctl
  manuels `0xff03`/`0xff05` supposent l'encodage amd64. Employer le sockopt
  fourni et préserver les bits architecturaux `_IO` pour les deux opérations.
  Pas d'exécution sur une autre architecture ; ne pas annoncer de portabilité.
- Preuve de transfert : confirmer explicitement que le pair de la connexion
  reste B, en plus de la preuve positive d'attribution à A avec son octroi.
- Fermeture : le dernier refus suit aussi une modification Trust. Vérifier un
  motif de fermeture distinct pour ne pas masquer une régression par la révision
  Trust déjà périmée.

Les limites déjà déclarées ne sont pas supprimées : autorité et octrois de
fixture, table de liaisons non persistante, aucun lanceur ou effet métier raccordé,
pas d'attestation par thread ni de protection contre le compte contrôleur hôte
non confiné. Le portage ne ferme pas F01–F07.
