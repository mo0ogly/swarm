# Démonstration Swarm pour LinkedIn — V2

Livrable : **Swarm_graphe_LinkedIn_FR_V2.mp4**, environ 63 secondes, format vertical 1080 × 1350 (4:5), H.264, sans son. Explications françaises incrustées et sous-titres SRT séparés. Aperçu GIF animé provenant du même enregistrement.

## Parcours

- Zoom et repli de branches dans le graphe.
- Ouverture du détail d’une tâche et lecture de ses prérequis non validés.
- Choix de la validation structurée ; lecture du critère complet.
- Saisie des fichiers, du programme `python3`, des arguments et du délai configurable.
- Association du critère au contrôle puis annulation de l’exemple.

Les images sont capturées pendant les interactions réelles dans le navigateur, avec leurs temps enregistrés. Le montage comporte des coupes et des agrandissements ; ce n’est pas une capture native à 25 images/s. La capture est échantillonnée à environ 3–5 images/s puis encodée à 25 images/s. Les champs et les changements viennent du produit : aucune interface ni exécution d’agent n’est animée artificiellement. Un seul thème est utilisé.

Le projet Casa Pizza est isolé : aucun agent lancé, aucun test exécuté, aucune acceptation simulée. L’exemple de configuration est annulé. Aucun changement de la mission réelle ou du serveur 18792.

Montage : `build_v2.py` (Pillow, ffmpeg, DejaVu). Les captures horodatées sont conservées localement dans `/home/fpizzi/workspace/rapports-banc/swarm-linkedin-v2-take2`, avec le journal par segment. `manifest-v2.json` contient leurs empreintes et celles des livrables ; `timeline-v2.json` précise les cadrages et légendes. Le montage est reproductible avec ce dossier source, qui n’est pas intégré au dépôt.

La V1, faite de neuf images fixes, reste conservée pour l’historique. Son audit est dans `AUDIT.md`. La vérification de la V2 est dans `AUDIT-V2.md`.
