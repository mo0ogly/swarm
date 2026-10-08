# Programme de recherche : médiation déterministe des agents LLM sur processus à effets irréversibles

Statut : document de cadrage du 7 octobre 2026, rédigé par Fabrice Pizzi pendant la campagne de
vérification des correctifs du moteur. Il rassemble le travail mené du 5 au 7 octobre 2026
comme base possible d'une thèse. Chaque résultat renvoie à son fichier de données ; les résultats
en cours de mesure sont signalés comme tels.

---

## 1. Problème

Des agents fondés sur des grands modèles de langage commencent à préparer des opérations dont
l'effet est irréversible : paiement, facturation, transaction. Les organisations d'agents
décrites par l'industrie (Cursor, *Towards self-driving codebases*, 2026) tolèrent une part
d'erreur et la corrigent après coup, ce qui convient au code, réversible. Pour un paiement, une
revue finale arrive après l'effet.

**Thèse défendue.** La sûreté d'un système d'agents qui agit sur le monde ne peut pas reposer
sur la qualité des réponses du modèle ni sur une revue finale. Elle doit reposer sur un
mécanisme déterministe qui médiatise chaque lancement, chaque reprise et chaque acceptation,
selon le principe du moniteur de référence (Anderson, 1972) : les agents proposent, le moteur
autorise, un exécutant déterministe agit. Ce contrôle a un coût, qui doit être mesuré.

## 2. Questions de recherche

| Question | Énoncé | État |
|---|---|---|
| QR1, sûreté | Quels contrôles d'exécution empêchent les effets doubles, les acceptations sans preuve valide et les reprises sans cause corrigée ? | Répondue sur un scénario (section 4) |
| QR2, coût | Quel surcoût ces contrôles imposent-ils ? | Durée mesurée ; blocages à tort (H6) non mesurés |
| QR3, attribution | Peut-on distinguer automatiquement une faute de l'agent, de l'environnement, de la règle et du moteur ? | Protocole pré-enregistré ; corpus non constitué |
| QR4, coordination | Les garanties tiennent-elles quand la coordination elle-même est confiée à des modèles ? | Conçue (S-collectif) ; non implémentée |
| QR5, correction du moniteur | Comment un moniteur de référence se met-il en défaut, et comment le vérifier ? | Six constats, correctifs en cours de vérification |

## 3. Contributions

| N° | Contribution | Preuve | État |
|---|---|---|---|
| C1 | Modèle d'états, huit invariants de médiation (I1 à I8) et trois propriétés de l'exécutant (E1 à E3) | `article-scientifique/brouillon-section-4.md` | Rédigé ; vérification formelle non faite |
| C2 | Implémentation ouverte (Swarm, Go, SQLite embarqué) et correspondance invariant → test | dépôt `mo0ogly/swarm` ; annexe B de la section 4 | Rédigé |
| C3 | Banc de facturation à injection de fautes, protocole pré-enregistré, résultats avec et sans moteur | `benchmarks/billing/`, `docs/benchmarks/billing/` | Mesuré (9 000 + 90 exécutions) |
| C4 | Taxonomie d'attribution des blocages, étendant MAST au moteur et à la règle | `brouillon-section-7-protocole.md` | Protocole seul |
| C5 | Boucle mesure → défaut → correctif → re-mesure appliquée au moniteur lui-même | `defauts-moteur.md`, `verification-correctifs.md` | En cours |

## 4. Résultats acquis

Source : `docs/benchmarks/billing/resultats/campagne-20261005.jsonl` (9 000 exécutions, agents
scriptés, empreintes du banc et du binaire inchangées du début à la fin) et
`lot-reel-20261006.jsonl`, `lot-reel-F8-20261007.jsonl` (90 exécutions, Claude Sonnet).

1. **Sûreté (QR1).** Sous le moteur, aucun paiement inexact ni faux succès sur 2 968 exécutions
   valides (Wilson 95 % : [0 ; 0,13 %]). Sans moteur, 1 900 succès déclarés démentis par le grand
   livre sur 3 000 ; un contrôle appel par appel n'en arrête aucun.
2. **Complémentarité.** La clé d'idempotence métier supprime seule les doublons ; le moteur
   supprime les paiements de candidats modifiés ou périmés et les faux succès. L'arrêt d'un
   candidat modifié se partage : avant le départ du règlement, le moteur (preuve non fraîche) ;
   après, l'exécutant (empreinte).
3. **Coût (QR2).** Durée multipliée par 8 à 30 sans faute.
4. **Agent réel.** Aucun effet faux sur 90 exécutions. Un workflow fixe reprenant les mêmes
   contrôles fait jeu égal avec le moteur sur les fautes testées : l'apport propre du moteur
   (concurrence, bail, reprise durable) n'est démontré qu'avec des agents scriptés.
5. **Isolement.** Le mode restreint du client laissait lire un fichier hors de l'espace de
   travail par `curl file://` ; d'où la séparation des profils d'outils (préparateur sans réseau).
6. **Le moniteur comme source de faute (QR5).** Cinq défauts et un choix de sûreté relevés
   (D1 à D6), dont un verrou SQLite non réessayé, seule cause des exclusions, et une couverture
   des exigences vérifiée à la clôture plutôt qu'avant l'effet.

## 5. Méthode

Ce qui rend ces résultats défendables tient autant à la méthode qu'aux chiffres :

- **Pré-enregistrement daté** dans git, avant chaque campagne : hypothèses, critères, règles
  d'exclusion, verdicts calculés par des modules d'analyse figés avant les données.
- **Amendements datés, jamais silencieux.** Plusieurs amendements datés dans les protocoles,
  dont deux après observation (lecture a posteriori des réponses d'agents ; défaut du banc sous F8 et rejeu),
  publiés avec les séries invalidées.
- **Contrôles positifs et négatifs.** Chaque faute doit d'abord produire son défaut sans
  protection ; sinon l'exécution est invalide. Le correctif D6 est vérifié avec un contrôle
  positif (sans la règle, le règlement part trop tôt).
- **Mesure dans le grand livre, jamais dans le discours des agents.**
- **Provenance.** Commit, état du dépôt, empreintes du banc et du binaire dans chaque campagne.
- **Comparaisons isolantes.** W-réel (workflow fixe) isole l'apport du moteur de celui des
  contrôles ; les conditions S-collectif-garde et -libre isoleront celui de la règle D6.

## 6. Ce qui manque pour une thèse

Un article est à portée ; une thèse demande davantage. Manques identifiés :

1. **Généralité.** Un seul scénario métier, une API simulée. Il faut au moins un second domaine
   à effet irréversible (livraison, provisionnement d'accès, publication) et une API réelle en
   bac à sable.
2. **Agents réels à l'échelle.** Cinq essais par scénario, un modèle. Il faut plusieurs modèles,
   plusieurs fournisseurs, des fautes propres aux modèles réalistes (la facture piégée testée
   a été déjouée par le modèle lui-même).
3. **Coordination par des modèles (QR4).** Étude S-collectif : conçue, non menée.
4. **Attribution (QR3).** Corpus, étiquetage, comparaison au diagnostic du moteur.
5. **Fondements formels.** Les invariants sont énoncés et testés, non prouvés. Une
   spécification (TLA+ ou modèle équivalent) des invariants I1, I2 et I4 sur petites instances
   est prévue par le plan (§ 4.5) et non faite.
6. **Validité externe et revue.** Auteur unique du moteur, des règles et du banc ; second
   annotateur humain absent ; aucune revue par les pairs à ce jour.
7. **Positionnement.** Les travaux liés (ACRFence, Safe to Resume?, RAILS, MAST, AgentSpec,
   CaMeL…) sont lus et situés ; une thèse demanderait un état de l'art complet sur la médiation
   à l'exécution, les transactions et la sûreté des agents.

## 7. Feuille de route proposée

| Étape | Objet | Dépend de |
|---|---|---|
| 1 | Verdicts de la campagne de vérification des correctifs (D1 à D6) | en cours |
| 2 | Préprint arXiv de l'article (C1 à C3, C5) | étape 1 |
| 3 | QR3 : corpus conservé, étiquetage, accord humain–modèle | — |
| 4 | QR4 : S-collectif garde et libre, pilote puis lot | étape 1 |
| 5 | Spécification formelle des invariants I1, I2, I4 | — |
| 6 | Second domaine et API réelle en bac à sable | étapes 2 à 4 |
| 7 | Fautes propres aux modèles réalistes, plusieurs modèles | étape 4 |

## 8. Documents de référence

| Document | Contenu |
|---|---|
| `article-scientifique/PLAN.md` | Plan détaillé de l'article |
| `article-scientifique/brouillon-*.md` | Sections rédigées (résumé, 2 à 7, 9 à 11, annexes) |
| `article-scientifique/fiches-lecture.md` | Travaux liés vérifiés |
| `docs/benchmarks/billing/conception.md`, `plan-implementation.md` | Conception et réalisation du banc |
| `docs/benchmarks/billing/observations.md`, `defauts-moteur.md` | Comportements du moteur et défauts |
| `docs/benchmarks/billing/verification-correctifs.md` | Vérification des correctifs (pré-enregistrement) |
| `docs/benchmarks/billing/conception-collectif.md` | Étude S-collectif |
| `docs/benchmarks/billing/resultats/` | Données brutes de toutes les campagnes |
