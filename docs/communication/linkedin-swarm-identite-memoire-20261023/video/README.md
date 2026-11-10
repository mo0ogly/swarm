# Vidéo Swarm supervision identité et mémoire

23 octobre 2026. [MP4 final](../Swarm_Supervision_Identite_Memoire.mp4) ·
[Sous-titres SRT](../Swarm_Supervision_Identite_Memoire.srt) ·
[Article et Word](../README.md).

**Vidéo produite : 120 secondes, 1920 × 1080, 30 images/s, H.264/yuv420p.**
Sous-titres français intégrés, sans voix, sans musique et sans capture du logiciel.
Les schémas se dévoilent progressivement ; le montage comporte des mouvements
légers et des fondus. Le fichier est préparé pour une lecture MP4 avec `faststart`.

This is a completed, silent French captioned explainer, not a screen recording
or a qualified live demo. No Swarm application behavior or shared contract changed.

## Chapitres du montage produit

| Temps | Contenu |
|---|---|
| 00:00–00:12 | Présentation de Swarm : supervision, identité et mémoire |
| 00:12–00:28 | Superviseur racine, sous-superviseur et exécutant |
| 00:28–00:44 | Observation, décision motivée, vérification et application |
| 00:44–01:02 | Parcours CLI Rust déjà vérifiés sur fournisseurs de fixture |
| 01:02–01:16 | Initiateur, agent logique, instance et autorité bornée |
| 01:16–01:34 | RETEX, mémoire versionnée, sélection et contexte reçu |
| 01:34–01:48 | AC-02 : confinement, réseau/secrets et traçabilité futurs |
| 01:48–02:00 | Sources publiques et état de maturité |

Le minutage est éditorial. Il ne mesure ni le temps réel d’une mission, ni les
performances de Swarm. Les vues sont explicatives ; aucun faux terminal ou écran
de produit livré n’est utilisé. La supervision occupe 50 secondes, puis reste
reliée aux sujets identité et mémoire.

## Sources et portée

Les affirmations Swarm reprennent les contrats et preuves reliés dans le
[suivi éditorial](../README.md). La carte Rust cite les résultats locaux 213,
224 et 244, relus comme PASS dans leur portée déclarée, sans les relancer.
Ils ne prouvent ni qualification globale, ni autonomie complète, ni appel à un
fournisseur IA réel dans cette vidéo. Les états Go, Rust en intégration et AC-02
non démarré sont indiqués à l’écran.

Références publiques présentées dans le générique :

- [Cursor — Scaling long-running autonomous coding](https://cursor.com/blog/scaling-agents), Wilson Lin, 14 janvier 2026.
- [NVIDIA — OpenShell Policy Schema Reference v0.1.5](https://docs.nvidia.com/openshell/v0.1.5/how-it-works/policies/schema).
- [Bromure — OpenShell policies and network lineage](https://bromure.io/en/blog/openshell-policies-and-network-lineage), Renaud Deraison, 21 octobre 2026.
- [Agent Baseline — The Secure Software Factory](https://github.com/agentbaseline/agentbaseline/blob/d842a1bb34a42e9d7c0d54a1b12ead9e0e59e283/software-factory/software-factory.md), version 1, 13 octobre 2026.
- [Agent Baseline — controls.yaml](https://github.com/agentbaseline/agentbaseline/blob/d842a1bb34a42e9d7c0d54a1b12ead9e0e59e283/whitepaper/controls.yaml), catalogue `1.0-draft`.
- [LangChain — Memory overview](https://docs.langchain.com/oss/python/concepts/memory).

Ces références éclairent les concepts et inspirations ; elles ne certifient
pas Swarm. Les illustrations sont produites depuis les sources Mermaid locales,
sans reprendre les captures Bromure dans le montage.

## Sources du média et vérification

[scenes.json](scenes.json) conserve titres, textes, durées et sous-titres.
Les quatre schémas sont modifiables : [supervision](supervision.mmd),
[conduite](conduite.mmd), [identité](identite.mmd) et [mémoire](memoire.mmd).
Mermaid 11.12.0 a été chargé localement ; le paquet de référence a été vérifié
par son intégrité SHA-512 du registre. Le navigateur de rendu n’avait aucun
accès réseau. FFmpeg a produit les huit séquences puis le MP4.

Le contrôle média vérifie 120 secondes, un flux H.264 1080p à 30 images/s,
3 600 images, absence d’audio et décodage complet sans erreur. Une image issue
du MP4 a été examinée pour chacun des huit chapitres. Les 24 sous-titres SRT
correspondent aux textes intégrés. Le rapport lie les entrées et le fichier
final par leurs empreintes.

Le scénario et le Word avaient été livrés avant le rendu du média. La demande
« la vidéo est où ? » a révélé cette confusion de livrable. Correction retenue :
produire le MP4, le décoder, vérifier ses images et livrer un lien vers le média.
Apprentissage pour les prochains contextes : distinguer explicitement scénario,
captures, montage encodé et vidéo d’application filmée ; une promesse de format
ou un document ne prouve jamais la présence du fichier attendu. Propriétaire :
Codex racine ; critère observé ici : MP4 final lisible, vérifié et lié dans l’index.

Aucun lot AC-02, build Swarm, test applicatif, appel fournisseur, service actif,
mission, base, budget ou permission n’a été modifié pour ce montage. Aucune
publication n’est effectuée. La capture CLI réelle reste une suite distincte,
conditionnée à la répétition d’un parcours public sur un binaire identifié.
