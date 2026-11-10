# Casa Pizza — tutoriels animés

Ouvrir `index.html` après extraction complète du ZIP. Le lecteur fonctionne sans serveur, fournisseur IA ni connexion Internet. Choisir un module, avancer étape par étape, ou ouvrir la vidéo courte. Les légendes sont intégrées aux MP4 ; les transcriptions et sous-titres VTT sont fournis. Le GIF est seulement un aperçu réduit. Aucun média ne démarre automatiquement.

Dans le cockpit installé avec cette édition : **Aide du cockpit → Formation Casa Pizza — tutoriels, guides PDF et kits**. Le lecteur et les téléchargements sont embarqués dans le binaire ; ils ne dépendent pas du dossier de la mission, de GitHub ou d’un service de vidéos. L’accès conserve l’authentification du cockpit. Un ancien binaire doit être mis à jour pour exposer cette entrée.

## Parcours

| Module | Durée | Guides |
| --- | ---: | --- |
| Besoin et graphe | 24 s | chapitres 8–11 et 17–20 |
| Contrôles Swarm | 24 s | chapitres 21–26 |
| Commande client | 30 s | chapitres 27–30 |
| Restaurant et suivi | 42 s | chapitres 31–34 |

Chaque module existe en FR et EN. Le lecteur fournit un exercice corrigé et relie les guides Word conservés, complétés par les chapitres 37 et 38. L’application de référence et les noms des tâches restent en français ; la narration écrite est bilingue.

## Nature des images et limites

Actions Swarm recapturées le 10 octobre 2026 avec le nouveau graphisme ; parcours client et restaurant enregistrés le 9 octobre 2026 dans un projet temporaire et des serveurs loopback isolés. Captures successives montées en séquences fixes ; ce ne sont pas des screencasts continus. Les captures client hautes sont recadrées dans les vidéos sur la zone utile, sans déformation ; le lecteur conserve les images complètes. Les barres de progression et légendes sont des ajouts éditoriaux. Les vidéos sont sans audio.

Le besoin enregistré et le plan pédagogique créé par le script sont deux démonstrations distinctes. Les six tâches restent non exécutées. Les contrôles sont prévisualisés puis annulés : aucune autorisation, aucun reçu ni avis d’acceptation inventé. La commande fictive est réellement créée et avancée manuellement côté restaurant. Aucun fournisseur IA n’a été invoqué pendant la capture ; cela ne signifie pas que créer les supports n’a consommé aucune ressource.

## Reproduction des médias

Depuis la racine du dépôt : `node tools/verification/training_ui_capture.cjs CHEMIN_BINAIRE DOSSIER_TEMPORAIRE` effectue les interactions dans un atelier jetable. Après réussite et inspection visuelle, copier ses PNG dans `frames/`. Avec Python, Pillow et ffmpeg installés, depuis ce dossier : `python3 build_graph_overview.py`, `python3 build_media.py --modules 01-plan 02-controles --swarm-recorded 2026-10-10`, puis `python3 build_player.py`. Les scripts ne pilotent pas le navigateur et ne reconstruisent pas une interface fictive : ils assemblent les images de `frames/`. `storyboard.json` relie chaque étape à sa capture. `manifest.json` contient les empreintes des captures et des médias. Les DOCX sont mis à jour par `extend_guides.py --originals-dir DOSSIER_DES_GUIDES_36_CHAPITRES` à partir d’une sauvegarde, puis rendus et vérifiés avant `python3 ../build_kits.py`.

# English

Open `index.html` after fully extracting the kit, then select English. The offline reader offers held screenshots, short captioned MP4s, GIF previews, transcripts and answered exercises. No autoplay, external dependency or AI provider is required.

In the updated cockpit: **Cockpit help → Casa Pizza training — tutorials, PDF guides and kits**. Assets are embedded in the executable and retain cockpit authentication. Older executables need updating.

These are edited real screenshots, not continuous screencasts. Tall customer captures are cropped proportionally in videos; full screenshots remain in the step reader. No audio. The teaching plan was created separately from saved requirements and remains unexecuted. Check policies were previewed and cancelled. A fictitious order was actually created and manually advanced; this is not proof of autonomous agents, real payment or real delivery.

PDF : Formation_Swarm_Casa_Pizza.pdf (FR), Swarm_Casa_Pizza_Training_EN.pdf (EN). Guides de 38 pages accessibles depuis le lecteur et inclus dans les kits ; Word conservés comme sources éditables.

## Présentation du graphe — interface du 10 octobre 2026

Le lecteur français propose également une capture montée des interactions réelles (63 secondes, 1280 × 900) : zoom, repli, détail des prérequis et paramètres de validation. Cette présentation est uniquement en français et masquée dans le lecteur anglais. Les quatre modules bilingues ne sont pas remplacés. Le MP4 et ses sous-titres sont embarqués et inclus dans les deux kits, pour permettre de changer la langue hors ligne. Aucun agent lancé ni contrôle exécuté dans cette présentation.

Les nouvelles captures concernent les tutoriels Swarm et leur présentation du graphe. Les captures des PDF et DOCX conservent leur date historique. Le plan demeure non exécuté et les paramètres montrés ont été annulés.
