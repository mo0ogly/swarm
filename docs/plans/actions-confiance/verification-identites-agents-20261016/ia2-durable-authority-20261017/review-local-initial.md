# Auto-revue locale — source durable

Codex, contexte producteur, non indépendant. Candidat du passage
`registry-after-run.json` : 30 noms PASS, pas une acceptation globale.

Verdict : **request changes**, retour au producteur.

1. Travail absent : le FK SQLite n'est vérifié qu'après écriture de l'intention
   Trust. Un scope visant un travail absent peut laisser une intention durable
   impossible à appliquer. Exiger existence du travail sous réservation SQLite
   avant cette écriture, avec un refus sans changement de Trust/journal. Cas à
   reproduire avant correction.
2. Les 30 contrôles couvrent le composant mais pas toute l'incidence du nouveau
   contrat `atomicWrite` et de la version 36. Ajouter les parcours existants
   stockage/inspection/migration, lancement, Trust et mémoire concernés ; aucune
   source mémoire à modifier. Les refus et versions doivent rester observables.
3. Le scan AST initial associe les appels par nom : il confond `os.File.Write`
   et les méthodes du moteur, et suit des appels postérieurs à COMMIT. Conserver
   le résultat comme surapproximation inutilisable pour prouver une inversion.
   Raffiner l'identification des receveurs et contrôler les frontières réelles.
   Les callbacks arbitraires restent une limite d'analyse, pas une preuve d'absence.

Limites de droit conservées : permission/checkpoint émis par le contrôle simulés,
aucun émetteur humain ou témoin externe déployé, aucun lanceur raccordé. Restaurer
tous les fichiers et fournir aussi un ancien témoin ne garantit aucune détection :
le caractère courant et indépendant du témoin reste une précondition de sécurité.
Le moteur en production conserve F01–F07 ouverts.
