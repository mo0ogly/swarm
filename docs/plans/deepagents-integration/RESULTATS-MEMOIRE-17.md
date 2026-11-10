# M21 — liaison des citations vérifiée, relais identité conservé

16 octobre 2026. Codex seul ; développement direct, `replan` et correction APEX
bornée. **PASS local pour M21 ; M20 et IA2/IA3 restent ouverts.**

## Résultat et périmètre

Le convertisseur mémoire refusait les textes altérés, mais acceptait une citation
renommée après recalcul des annotations et un échange répété sous d'autres
identifiants. Le préparateur de choix acceptait aussi ces alias si le reçu
extractif était reconstruit. Le validateur partagé vérifie désormais la liaison
de chaque identifiant au JSON de référence émis par Go et refuse la répétition.
Les deux CLI refusent avant création de sortie et conservent les entrées.

Les fixtures Python utilisaient des identifiants `q` / `a` non produits par Go.
Elles utilisent désormais le même format que le producteur réel. Quatre vecteurs
retenus depuis la bibliothèque JSON Go vérifient encodage, échappement et ancien
`attempt` vide. L'ordre des clés dans le paquet reçu ne change pas l'identité de
source reconstruite. L'ancien échange sans clé de réponse reste compatible ;
aucun auteur ou droit ne lui est inventé.

Fichiers applicatifs modifiés dans ce lot :
`tools/memory-evaluation/swarm_source.py` uniquement. Les deux modules de tests
Python, le fichier de vecteurs et la documentation/suivi associés sont actualisés.
Le préparateur `admission.py` reçoit la correction par son appel existant au
validateur partagé ; son code reste inchangé. Aucune source Go, politique,
configuration Admin, base de mission ou identité n'est modifiée par ce lot.

## Preuves et contrôles

Paquet : [`evidence/m21-20261016/`](evidence/m21-20261016/). HEAD
`4e58f7cbf8200a58f69f1f4d03d657c3313fb642`, branche
`codex/clear-product-method-names`, avec changements existants conservés.
`baseline.json` conserve les dix préimages et les empreintes des treize sources
communes lues ; le SHA Git seul ne décrit pas les modifications locales.

| Critère | Vérification effectivement exécutée | Résultat / preuve |
| --- | --- | --- |
| Défaut avant correction | Douze tests convertisseur : trois méthodes nouvelles échouent sur cinq variantes ; sept tests de choix : deux nouvelles méthodes échouent. Les contrôles précédents restent verts. | Échec attendu conservé dans `before-source.*` et `before-selection.*` ; pas un PASS de protection. |
| Refus partagé et non-régression | `python3 -m unittest discover -s tests -p 'test_memory_s*.py' -v` | 57 méthodes nommées passent : 49 précédentes et huit nouvelles ; première sortie dans `python-targeted.*`, résultat final lié aux empreintes dans `python-final.*`. |
| Concordance Go/Python | Quatre vecteurs sérialisés/hachés par `encoding/json` et `crypto/sha256`, puis testés côté Python. | `source-id-vectors.*` et `tests/memory_summary_source_id_vectors.json`. Ce sont des vecteurs d'encodage, pas des admissions d'agents. |
| Parcours public de bout en bout | `python3 tests/memory_summary_admission.py BINAIRE_M19 SORTIE_NOUVELLE` | Cinq contrôles M19 et cinq contrôles M18 passent, trois invocations scriptées, zéro appel de modèle ; `native.json`, `native/result.json`, `native/source-flow/result.json`. |
| Sortie et refus | Les deux CLI rejettent une citation renommée avec sortie 2, sans fichier/dossier de sortie, sans modification des entrées et sans traceback. | Deux tests CLI nommés dans le groupe Python ; les sorties valides restent privées et ne remplacent pas un fichier existant. |
| Diff, documentation et séparation | Contrôle global `git diff --check`, fins de ligne des fichiers possédés, liens relatifs et empreintes des contrats Go. | PASS : 19 liens locaux résolus, 13 contrats communs inchangés ; `owned-changes.patch` / `.json` et `audit-final.json`. Aucun fichier du chantier identité modifié. |

Le parcours public utilise le binaire M19 conservé
`/tmp/swarm-m19-bin-o0uuhvjr/swarm`, SHA-256
`026fcf377d64cd43951e5c7fc3a6d20ee5ea42c9323e3ffeaf65b1566895f7e2`.
La correction est dans les outils Python réellement exécutés par la recette.
Les sources Go concernées restent inchangées ; aucune nouvelle compilation Go
n'est requise pour cette correction hors moteur. Cette recette ne qualifie pas
les changements de consignes embarquées intervenus depuis M19, ni un nouveau
binaire, ni un déploiement du serveur utilisateur.

Les reçus détaillés lient les entrées Python, la recette et le binaire à leurs
empreintes avant/après. Les fixtures sont des racines temporaires isolées ; les
fournisseurs et le serveur démarrés par la recette sont ceux de cette fixture.
Les résultats M19 et IA1 historiques ne sont ni remplacés ni renotés.
Le serveur utilisateur est encore prêt sous le PID 2186583, à
`http://127.0.0.1:38433/`, lors du contrôle final en lecture seule ; il n'a pas été
redémarré ni remplacé par cette reprise.

## RETEX et améliorations conservées

La cause était une validation incomplète du format réellement produit, masquée
par des fixtures plus permissives. Corriger chaque CLI aurait dupliqué la règle.
La correction du validateur commun garde le même schéma et atteint les deux
consommateurs existants. Les nouvelles régressions incluent les alias malgré
empreintes concordantes, la répétition, le refus sans sortie, l'ordre des clés,
l'ancienne clé vide et l'encodage Go.

Incident de méthode conservé : certaines lectures terminal trop larges ont été
tronquées et une recherche a cité un nom de fichier inexistant. Aucune de ces
sorties n'est utilisée comme inventaire complet ni comme preuve PASS. Les
contrats utiles sont lus par symboles/fichiers existants ; les sorties de tests
et de recettes sont conservées intégralement. Un premier wrapper de tests
n'affichait que la sortie immédiate et perdait l'identifiant du processus ; le
reçu final a été relu avant toute annonce de réussite ou modification des sources.
Le premier reçu Python PASS ne liait pas les empreintes des entrées. Une dernière
exécution ciblée est donc justifiée par cette preuve manquante et enregistrée
séparément dans `python-final.*`, avec empreintes avant/après identiques ; la
sortie initiale est préservée. Aucun élargissement des tests Go n'est effectué.

L'[inventaire M20](AUDIT-PROVENANCE-M20.md) consolide treize voies de consommation,
dont historique automatique, assistant, brief, RETEX et import. Une empreinte
cohérente ne rend pas le texte authentique ou autorisé. L'identité de projet ne
doit pas être déduite du seul hash de chemin racine ; ce point est conservé pour
IA2/M20, sans inventer maintenant un nouvel identifiant ni une autorisation.

## État du besoin initial et condition de reprise

M21 est une correction indépendante du convertisseur, pas l'application du
propriétaire, de l'auteur et du droit d'usage demandée en M20. La conversation
« l'audit des identités » a livré sept contre-exemples IA1 sans correction des
droits. Ses sources communes restent en lecture seule dans cette reprise ;
aucun message inter-chat ni sous-agent n'est lancé.

Reste à recevoir d'IA2/IA3 : un principal et un destinataire authentifiés, une
décision d'usage applicable au contenu exact, des règles de délégation/révocation
et une frontière de contrôle protégée. Le chantier mémoire pourra alors appliquer
ces contrôles aux treize consommateurs et vérifier demande → décision → application
→ réception. Les questions de raccordement sont dans l'inventaire M20.
Le propriétaire du socle est le chantier identité ; celui des consommateurs est
ce chantier mémoire. Aucune permission implicite, ouverture hiérarchique,
qualification LLM, revue indépendante ou acceptation globale n'est revendiquée.
