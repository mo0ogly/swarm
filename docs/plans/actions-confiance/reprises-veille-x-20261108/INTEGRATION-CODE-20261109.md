# Intégration bornée — 9 novembre 2026

Mesure : 2026-11-08T23:37:13.318717+00:00. Cinq lots de code existant intégrés en séquence sur main, avec index temporaires isolés et sélection de hunks partagés ; fichiers de travail préservés.

- 01-focus : `7e8cca69076a60a7671d2179eec0f0f536392e8e`
- 02-badges : `9d40b7e60648756ed1082b7991f6288e0063bf51`
- 03-gi : `8549dcc42f8e924db76685a8d0d0d7292cefafca`
- 04-history : `1d69f956b7201f6266e773831700f3fb3b1da005`
- 05-qualification : `a547d8e0519e4bf5620842e991b535889e59be85`

Candidat v0.1.1-dev.365 construit normalement puis sélectionné par le redémarrage pris en charge. Empreinte image active `/proc/2400907/exe` : `77a26af00a792d0090cd220306468e26e6f1fde5caa0b1f1641ba44a65c9fa6c`. Build lié à a547d8e0519e4bf5620842e991b535889e59be85, modifié=true ; cette provenance ne sera pas celle d'un futur commit documentaire. Les chemins intégrés n'ont pas de dérive face à HEAD au contrôle.

## Vérification et portée

Recette Focus : 5 cas nommés ; badges : 4 cas agent et 5 cas tâche. Résultats et vérification de preuves PASS. Refus initial Focus avant test conservé ; précondition de répertoire corrigée, nouvelle sortie distincte. Les preuves moteur GI/361/364 sont réutilisées dans leurs entrées exactes, sans prétendre qualifier chaque arbre Git intermédiaire. Sur les 782 entrées déclarées comparées à364, seuls HELP_EN dans lib.rs et les notes version_history changent. L'aide EN omettait review-refresh : ajout à l'inventaire d'aide, sans modifier dispatch, droits ou politique. Aide FR/EN du binaire365 relue. Candidat et profil natif par défaut vérifiés dans `.cache/local-delivery/rust-tm0ucj0m` ; aucune fixture activée dans ce profil selon preuve du builder.

Le serveur365 est actif et prêt. La fenêtre Version FR/EN, ses notes et lien Git, le résumé calculé et la lecture historique dans la session réelle365 restent NON VÉRIFIÉS : le frontdoor38433 redirige le navigateur vers la connexion. Aucune extraction de secret, contournement ou nouvelle recette sur données actives. Une connexion utilisateur est demandée ; ne pas annoncer l'acceptation visuelle.

## Revue, RETEX et suite

Inventaire précis, blobs committés, empreintes et revue politiques/fonctions/erreurs/limites : INTEGRATION-CODE-20261109.json. Les revues antérieures sont bornées à leurs contrats ; absence de nouveau test moteur n'est pas acceptation indépendante. Le défaut d'aide a été reproduit sur364 avant correction, puis présence FR/EN observée sur365. Apprentissage : lire aussi les deux inventaires de commandes, pas seulement le dispatch.

GitHub exige une PR avec checks/install stricts, admins soumis ; zéro approbation humaine exigée par la protection relue. Décision manquante : autoriser une branche de livraison unique et sa PR, alors que les consignes actuelles interdisent les nouvelles branches. Aucun push inchangé répété ni protection abaissée. N5 et les scénarios natifs non remis restent ouverts. Facturation différée.

Lexar réellement montée mais presque pleine (149Mo libres observés) : nouvelles preuves conservées localement dans `.cache/integration-code-20261109`, disque local21Go libres. Aucun original supprimé. Deux historiques de chats refusent source rollout manquante ; cause USB non établie. Une recherche USB de noms a été interrompue, donc n'établit aucune absence. M33 récupéré à partir de son pointeur d'archive reste distinct d'une restauration des chats.


## Publication par branche de livraison — 2026-11-09T06:06:30.523227+00:00

Autorisation explicite reçue : une seule branche de livraison et une PR, sans changement de protection ; développement sérialisé sur main, facturation toujours différée. Branche codex/delivery-swarm-20261109 poussée au SHA79b8d854a8e1aab4cc5d6f896b6cdb116918a34a ; PR https://github.com/mo0ogly/swarm/pull/22 ouverte. Huit commits locaux existants publiés sur cette branche ; aucun dirty étranger inclus. Main local reste sélectionné, main distant b4e07cb26066cffdb20b36ac2fc9c60f758cb23e au contrôle. PR/checks/install stricts et enforce_admins relus inchangés ; checks en cours, pas PASS ni fusion. État et preuves : PUBLICATION-GITHUB-20261109.json. L'autorisation ne porte pas sur le merge ou une autre branche.

Connexion désormais disponible : fenêtre Version365 réellement observée FR/EN, notes correspondant au binaire, commit a547d8e0519e et lien GitHub exact, disclosure modifié visible. Lecture mission historique w-ca937edf4e0abd85f8d3d67b : résumé révision881 avec T1 fresh/passed/accepted, T2 unknown/not_accepted ; passage historique explicite affiche « Historique reçu — statut opérationnel non évalué » avec bouton distinct Vérifier le statut actuel. Aucune tâche/agent/provider relancé. Observation de lecture, pas nouvelle acceptation indépendante, conformité globale ou test sur le datastore actif. Ces nouvelles observations lèvent seulement l'ancien blocage de connexion, sans effacer son historique. Preuves UI locales avec empreintes dans le record ; Lexar presque pleine, repli local conservé.

Suivi automatique existant actualisé et relu pour cette unique PR, sans autorité de merge. Aucun code ou contrat produit modifié par la publication ; documentation de livraison/coordination uniquement, pas nouveau build/test justifié. Anciennes recherches de noms supposés checks.yml/install.yml corrigées par inventaire réel .github/workflows/ci.yml ; contrôles nommés jobs, pas fichiers distincts. Les sélecteurs sémantiques absents ont été remplacés par ID observé/AX natif, sans prétendre à une correction produit.
