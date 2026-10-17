# Replan — capacité de contexte configurable

Le partage source/diff seul ne suffit pas (-6053 tokens). Répéter la consigne
ou augmenter les appels ne change pas le refus. Le client installé 0.159.2
annonce pour le modèle déjà configuré une fenêtre par défaut de 272000 tokens
et une fenêtre maximale de 872000. Le cache utilisateur est d'une autre version
(0.159.0) et n'est pas une preuve applicable au client exécuté.

Choix : conserver modèle, réserves et originaux ; ajouter une fenêtre de contexte
explicitement configurée par mission, persistée dans ReviewerConfig, présentée
et modifiable par la même fenêtre Admin, API et CLI de quotas. Zéro conserve le
défaut du client ; aucun sens « illimité » pour une fenêtre physique. Un contrat
de capacité versionné lie défaut, maximum annoncé, valeur demandée, fournisseur,
modèle et empreinte de l'exécutable. Le client reçoit model_context_window à la
même valeur. Une demande au-delà du maximum est refusée sans mutation ni appel.

Vérifier : défaut historique inchangé, refus invalides sans mutation, aperçu et
sauvegarde/réouverture API/CLI/Admin FR/EN/thèmes/mobile, commande réellement reçue,
compteurs préservés, même dossier natif complet mesuré avant nouvelle dépense.
La correction métier change le candidat et exigera ses contrôles et une revue
fraîche ; aucun avis historique ne sera réutilisé pour accepter la nouvelle copie.
