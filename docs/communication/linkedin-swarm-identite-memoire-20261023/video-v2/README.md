# Film Swarm identité mémoire et contrôle humain

[Lire le MP4](../Swarm_Architecture_Film_v2.mp4) · [Word v6](../Swarm_Article2_Identite_Memoire_v6.docx) · [SRT FR](Swarm_Architecture_Film_v2.fr.srt) · [English SRT](Swarm_Architecture_Film_v2.en.srt).

Film local de 100 secondes, 1920 × 1080, 30 images/s, H.264/yuv420p, AAC stéréo.
Français à l’écran, musique électronique instrumentale originale, sans voix.
Les 27 sous-titres sont intégrés visuellement en français ; deux pistes texte
FR/EN et les deux fichiers SRT sont fournis. Les plateformes peuvent gérer ces
pistes différemment ; les SRT restent disponibles séparément.

Les flux, points lumineux, apparitions et fondus sont rendus à chaque image.
Le logo canonique reste intact. Marine, cyan et doré suivent UI-DESIGN. Aucun
faux terminal, résultat de test, écran de produit ou contrôle déjà livré n’est
fabriqué pour le film. Les extraits animés complètent les cinq schémas techniques
FR/EN ; ils ne les remplacent pas. AC02 reste enregistré et non démarré.

This is a French motion-design explainer with original instrumental audio and
French/English captions. It is not a screen recording or runtime qualification.
The five complete bilingual architecture views are preserved. No publication,
service restart, mission operation or AC02 implementation is performed.

## Chapitres

| Temps | Séquence |
| --- | --- |
| 00:00–00:10 | LA CONTINUITÉ A UN RISQUE |
| 00:10–00:22 | 01  MÉMOIRE |
| 00:22–00:34 | 02  IDENTITÉ |
| 00:34–00:46 | 03  MOTEUR DÉTERMINISTE |
| 00:46–00:58 | 04  VALIDATION HUMAINE |
| 00:58–01:10 | 05  SUPERVISION |
| 01:10–01:20 | 06  TRACES ET PREUVES |
| 01:20–01:30 | 07  ACTIONS DE CONFIANCE |
| 01:30–01:40 | SWARM |

## Sources et portée

Le [scénario](scenes.json) relie chaque scène aux identifiants du
[catalogue des contrats](../../../diagrams/architecture-contracts.json) ou à
l’[article sourcé](../article-v4-fr.md). Le contrôle humain s’appuie sur
[ENGINE-ACCEPTANCE](../../../ENGINE-ACCEPTANCE.md#contrôles-comme-preuves-dune-revue-humaine).
L’origine/integrité d’un souvenir reste distincte de sa vérité et de son autorité.
La maturité Rust est celle des parcours documentés, avec leurs limites ; les
animations ne sont pas des preuves d’une nouvelle exécution.

Les publications MINJA, MemoryGraft, Microsoft, MemGhost et Emergence World
sont citées dans le générique et reliées aux sources primaires dans les
[18 références](../article-v4-fr.md#liens-des-nouvelles-références) et le
[contrôle des dates et des affirmations](../sources-v4-review.json).

## Reproduire le rendu

Utiliser Node/Python et les bibliothèques retournés par le chargeur des dépendances
Codex. `render.mjs` utilise le canvas natif et les polices Liberation locales ;
aucune requête réseau ni dépendance produit n’est ajoutée. Depuis la racine :

```sh
NODE_BUNDLE docs/communication/linkedin-swarm-identite-memoire-20261023/video-v2/render.mjs --stills
NODE_BUNDLE docs/communication/linkedin-swarm-identite-memoire-20261023/video-v2/render.mjs --encode
PYTHON_BUNDLE docs/communication/linkedin-swarm-identite-memoire-20261023/video-v2/soundtrack.py
```

`NODE_BUNDLE` et `PYTHON_BUNDLE` désignent les exécutables du runtime retourné,
à remplacer par leurs chemins réels. Le rendu refuse les débordements de texte.
La composition sonore utilise uniquement des oscillateurs et une partition
originale ; aucun échantillon externe ni imitation d’artiste. Ses paramètres
sont des choix de présentation, pas des politiques Swarm.

L’assemblage final prend le MP4 silencieux, le WAV, les deux SRT et
[chapters.ffmetadata](chapters.ffmetadata), copie H.264, encode AAC et mov_text,
puis place l’index MP4 avant les données. Les arguments exacts et empreintes sont
conservés dans la [revue](../media-refresh-review.json). Ne pas remplacer un
ancien livrable tant que le nouveau candidat n’est pas vérifié.

## Vérification

[verify.py](verify.py) lit le fichier encodé, compte et décode toutes ses images
et son audio, vérifie flux, durée, chapitres, SRT et absence d’écrêtage numérique.
Il extrait trois images par chapitre pour inspection. Lancer avec un dossier
neuf ; les images et sorties brutes restent dans les preuves. Aucun watchdog
global arbitraire n’est ajouté. Les mesures audio ne remplacent pas une écoute
humaine ; l’appréciation du rythme et de la musique reste éditoriale.

La [revue finale](../media-refresh-review.json) distingue vérification du média,
relecture visuelle, Word rendu et conformité des contrats. Elle ne constitue
ni revue indépendante ni qualification globale du moteur.
