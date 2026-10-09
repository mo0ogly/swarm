# Audit du MP4 V2 — 9 octobre 2026

## Vérifications

- Capture horodatée pendant les interactions du navigateur : 217 images sur quatre segments montés. Les actions de zoom, repli, ouverture, saisie et sélection viennent réellement de l’interface.
- Format final : H.264, 1080 × 1350 (4:5), 63 secondes, encodage 25 images/s. Capture échantillonnée environ 3–5 images/s ; aucun enregistrement natif fluide à 25 images/s revendiqué.
- Décodage intégral ffmpeg sans erreur ; métadonnées vérifiées par ffprobe.
- Dix images du MP4 extraites aux secondes 1, 5, 10, 20, 27, 36, 42, 48, 53 et 60. Contrôle visuel par l’assistant, avec déclinaisons 540 × 675 ; ce n’est pas une revue humaine indépendante.
- Le critère est complet à 36 s, `python3` complet à 42 s, le délai et son champ complets à 48–53 s. Les arguments apparaissent pendant leur saisie et avant le passage au délai. Le détail des prérequis est agrandi.
- Les grandes légendes restent lisibles à taille réduite. Les libellés secondaires et la vue globale des six tâches demandent un agrandissement : aucune affirmation que tout le texte du cockpit soit lisible sur un petit téléphone.
- Aucun changement de thème, aucune exécution fournisseur, aucune acceptation inventée. La configuration est annulée. Les avertissements du produit ne sont pas altérés ; le cadrage se concentre sur les actions démontrées.
- Carte finale présente. La première exportation raccourcissait cette carte ; horloge d’entrée corrigée, contrôle répété sur le fichier remplacé.
- Programme auparavant coupé : cadrage élargi. Légendes réajustées aux états effectivement montrés, sans annoncer qu’un critère est coché avant son action.

## Portée de la livraison

Vidéo pédagogique de navigation et de configuration du cockpit, avec montage et plans rapprochés. Elle ne démontre pas une mission autonome exécutée jusqu’à la clôture. Le rythme reste celui d’une capture échantillonnée, avec des coupes entre segments. Sans voix off ; les explications sont incrustées et disponibles en SRT.

La compression après publication LinkedIn et le visionnage sur téléphone physique n’ont pas été vérifiés. Le contrôle à taille réduite est une simulation locale, pas une recette de l’application LinkedIn.

Sources, horodatages et images d’audit : `/home/fpizzi/workspace/rapports-banc/swarm-linkedin-v2-take2`. Empreintes : `manifest-v2.json`. La V1 et son audit restent conservés. Aucun commit, push ni publication effectué.
