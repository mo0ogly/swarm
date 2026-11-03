# Reference guide — Avant vérification et livraison du modèle

Les chemins et commandes conservent leur contexte de travail d’origine.

## Vérifier et livrer

Choisir une racine isolée et un binaire construit depuis le candidat. Ne pas
modifier un candidat figé de campagne, une base de mission vivante ou les fichiers
d’un agent actif. Les budgets, tentatives, refus et historiques restent conservés.
Un aperçu isolé permet de présenter le résultat sans redémarrer le cockpit vivant.

Pour un modèle exécutable, vérifier via les opérations publiques : adaptation et
adoption, références et cycles, refus d’entrées invalides, prérequis non satisfaits,
chemin de refus, reprise après précondition corrigée et validations sur le même
candidat. Contrôler les branches en observant les actions réellement autorisées
ou refusées, pas seulement le dessin. Les fixtures et agents scriptés restent
identifiés ; ne pas prétendre avoir testé un LLM réel.

Pour l’UX, vérifier FR/EN, deux thèmes, mobile, Tab/Maj+Tab, Échap et retour du
focus, chargement, erreur/reprise et conservation des réponses. Collecter les
erreurs navigateur et les requêtes dès le départ. Distinguer les visites de la
fixture d’initialisation des mutations du parcours testé. Inspecter les captures,
scanner les jetons/couleurs et exécuter les contrôles frontend concernés.

Exécuter les tests moteur proportionnés si le moteur change, puis les contrôles
requis par le dépôt. Une modification d’explication frontend ne justifie pas de
rejouer une campagne globale de facturation. `git diff --check` et inspection du
diff restent nécessaires. Rafraîchir un manifeste courant uniquement selon la
procédure du dépôt ; cela ne renouvelle pas une acceptation historique.

Livrer le modèle et son mode réel (cadrage, travail exécutable ou proposition),
les critères vérifiés, le candidat, les résultats et les limites. Une auto-revue
n’est pas une revue indépendante. Commit, push et publication suivent les
instructions de l’utilisateur ; la skill n’accorde aucune autorisation supplémentaire.
