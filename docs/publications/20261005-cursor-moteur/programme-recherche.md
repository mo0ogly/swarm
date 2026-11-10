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

Corpus moteur actualisé le 10 octobre 2026 à la demande de l’utilisateur : Cursor en priorité,
avec janvier, février, avril, juillet et septembre ; Google/Gemini, OpenAI et Anthropic en
comparaison. Voir les [fiches de lecture et essais proposés](article-scientifique/fiches-lecture.md#5-moteur-swarm--sources-industrielles-actualisées-au-10-octobre-2026)
et l’[état de l’art actualisé](dossier-recherche/02-etat-art.md#26-actualisation-du-corpus-moteur--10-octobre-2026).
Cette extension du corpus ne change pas les résultats déjà mesurés ni les critères d’acceptation.

Observation moteur ajoutée le 10 octobre 2026 : les plafonds fixes de 90 secondes
coupaient la revue et le diagnostic du responsable. La correction distingue
activité, silence, durée totale facultative et bail renouvelé, avec réglages
persistés dans Admin. La reprise réelle a produit un verdict puis une décision de
correction automatique. Voir le [chapitre sur le moniteur](dossier-recherche/07-moniteur.md#76-vivacité-des-superviseurs--observation-du-10-octobre-2026)
et la [note de recherche datée](dossier-recherche/revisions/20261010-vivacite-supervision.md).
Ce cas hors campagne ne modifie pas les résultats de facturation ni les causes
inconnues des cinq délais historiques.

Autre observation du 10 octobre : deux refus `unknown` persistent parce que les
citations sont transmises sans leur source complète. La [note sur le dossier de revue](dossier-recherche/revisions/20261010-refus-contexte-revue.md)
étudie la correction supervisée insuffisante, la réparation externe explicite et
les mesures à pré-enregistrer. Ce cas ne requalifie aucun verdict historique.

## 2. Questions de recherche

Extension du 16 octobre 2026 : [identité et autorité pendant tout le cycle de vie](article-scientifique/identites-autorite-cycle-vie.md).
Elle propose QR-ID1–4, une comparaison avec contrôles équivalents et trois
schémas. Les précédents scientifiques interdisent de présenter l'identité ou
le contrôle continu comme nouveaux en eux-mêmes. L'audit IA1 et l'expérience
d'isolation IA2 restent distincts d'une intégration ou d'une évaluation complète.
Les tableaux historiques ci-dessous gardent leur périmètre de facturation/moteur.

Actualisation locale du 17 octobre : la même note relie le noyau privé Go IA2.2
(114 noms de tests/sous-tests réussis, dont 100 propres au noyau) et un prototype
Linux séparé (13 contrôles réussis). Le transfert réel d'une connexion entre
instances fournit un cas d'attribution à reproduire au futur courtier. Aucun
lanceur n'est raccordé ; F01–F07 restent ouverts. Ces preuves de composants ne
répondent pas encore à l'évaluation comparative QR-ID1–4 et n'établissent aucune
priorité scientifique.

Le portage Go IA2.3a ajoute depuis une attribution réelle au vérificateur interne :
133 noms de tests/sous-tests ciblés réussis, dont 19 nouveaux, avec répétition de
ces 19 sous `-race`. Le dossier identité lie le cas d'expiration durant l'attente
du registre à son échec initial et à sa correction. L'autorité demeure une fixture
dans ces essais ; les lanceurs et les effets métier ne sont pas raccordés.

La source durable IA2.3b lit depuis SQLite sous une tête Trust et un checkpoint
distincts ; une publication interrompue est reprise sans doublon. 181 noms de
tests/sous-tests ciblés passent, dont 33 du registre, avec 52 répétitions sous
`-race`. Le témoin courant et le permis restent émis par les fixtures. Ces
résultats préparent la réplication du protocole ; ils ne qualifient ni l'admission
publique, ni un témoin externe déployé, ni une contribution originale démontrée.

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
| `dossier-recherche/revisions/20261010-vivacite-supervision.md` | Coupures de durée, rôle du superviseur, correction, reprise réelle et protocole de comparaison proposé |
| `dossier-recherche/revisions/20261010-refus-contexte-revue.md` | Deux refus par manque de source complète, limite de la correction supervisée, réparation explicite et essai comparatif proposé |
| `dossier-recherche/revisions/20261010-correction-globale-preuves.md` | Transmission automatique des preuves, inventaire aux superviseurs, vérifications et installation ; schémas d’architecture FR/EN |
| `docs/benchmarks/billing/conception.md`, `plan-implementation.md` | Conception et réalisation du banc |
| `docs/benchmarks/billing/observations.md`, `defauts-moteur.md` | Comportements du moteur et défauts |
| `docs/benchmarks/billing/verification-correctifs.md` | Vérification des correctifs (pré-enregistrement) |
| `docs/benchmarks/billing/conception-collectif.md` | Étude S-collectif |
| `docs/benchmarks/billing/resultats/` | Données brutes de toutes les campagnes |

Le [raccord IA3a au contrat enregistré](../../plans/actions-confiance/verification-identites-agents-20261016/ia3-plan-binding-20261017/RAPPORT-IA3a.md)
ajoute une génération persistée à chaque amendement, y compris suivi du retour
à un ancien contenu. Une nouvelle course entre lecture SQL et verrou runtime
est reproduite puis corrigée par sérialisation de la décision pure. 232 noms
ciblés passent, dont 32 du plan ; ils ne constituent ni 232 attaques distinctes
ni un résultat comparatif. Les émetteurs humains, le témoin du watermark de
plan, les effets publics et la mesure des coûts restent ouverts. Les précédents
et l’absence de preuve d’originalité sont inchangés.

Le [producteur interne IA3b](../../plans/actions-confiance/verification-identites-agents-20261016/ia3-issuer-20261017/RAPPORT-IA3b.md)
relie une session OIDC à une autorité explicite de publication. Les expériences
locales distinguent transfert entre sessions, expiration pendant attente et
lecture du travail avant autorisation. Enrôlement et témoin courants restent
des prérequis ; IA3c.1 traite désormais le reçu historique durable dans le noyau
privé, avec résultats et limites dans son rapport. Aucun
résultat de sécurité des lanceurs publics ou de nouveauté scientifique n'en
découle. La documentation doit accompagner chaque modification : règle persistée
pour Codex/Claude et assemblage du pack contrôlé, sans preuve de déploiement.


Le [reçu durable IA3c.1](../../plans/actions-confiance/verification-identites-agents-20261016/ia3-issuance-receipt-20261017/RAPPORT-IA3c1.md)
lie l’autorité fournie, la session sous empreinte et les dates à la tête du
registre. Les tests couvrent réouverture, altération et reprise ; l’autorité
courante et la preuve d’enrôlement restent distinctes de ce reçu historique.
Les résultats sont des fixtures locales, sans compte humain externe ni admission
publique. Aucune nouveauté scientifique ou supériorité comparative n’est établie.


IA3c.2a ajoute un format de réponse signée isolé, avec clé épinglée, challenge
et anti-rejeu local ; il n’est pas raccordé au moteur. Le
[contrat d’enrôlement cible](../../plans/actions-confiance/identites-agents-enrolement-IA3c2.md)
conserve explicitement origine authentifiée versus autorité courante, décisions
de déploiement et exigences de révocation. Aucune nouveauté scientifique n’est
déduite de ces mécanismes classiques ni des tests du candidat.


IA3c.2b ajoute un [journal local d’enrôlement](../../plans/actions-confiance/verification-identites-agents-20261016/ia3-enrollment-state-20261017/RAPPORT-IA3c2b.md)
avec chaîne signée et révocation irréversible, testé en normal/race. Le témoin
externe reste fourni, sa protection durable non réalisée. La lecture active
est sérialisée localement ; la fraîcheur au moment d’un usage moteur et la
révocation distribuée restent ouvertes. Le rapport conserve l’échec concurrent
initial, sa limite de diagnostic et les résultats ultérieurs sans les confondre
avec une validation comparative ou une nouveauté scientifique. Le futur protocole
d’évaluation doit mesurer aussi coût de relecture de l’histoire, contention,
retour arrière du témoin et délai entre réponse authentique et usage effectif.


IA3c.2c apporte un [schéma local strict d’autorité](../../plans/actions-confiance/verification-identites-agents-20261016/ia3-enrollment-authority-20261017/RAPPORT-IA3c2c.md)
et une génération liée à l’événement de création du journal. La validation
producteur/consommateur réutilise les primitives moteur. Les tests distinguent
signature, cohérence, contexte attendu et fraîcheur ; l’acceptation d’une ancienne
preuve avec son ancien témoin reste une limite explicite. Demande complète EP-02,
service protégé, intégration publique et mesure comparative restent ouverts.
Aucune nouveauté scientifique ou garantie globale ne découle des tests locaux.


IA3c.2d lie une [demande complète](../../plans/actions-confiance/verification-identites-agents-20261016/ia3-enrollment-request-20261017/RAPPORT-IA3c2d.md)
à son challenge : candidat entier, contexte humain/session, plan, événement,
racines, checkpoints et document exact. Le protocole expérimental doit distinguer
couverture du contenu, provenance vérifiée des faits, actualité et autorité d’usage.
Les tests locaux ne qualifient ni le service protégé ni le workflow public ;
aucune nouveauté scientifique n’est déduite du digest avec domaine/version.


IA3c.2e ajoute une [version explicite au cadre de lecture](../../plans/actions-confiance/verification-identites-agents-20261016/ia3-enrollment-reply-20261017/RAPPORT-IA3c2e.md)
et un décodeur commun. Les paquets authentiques non reconnus sont refusés sans
fallback ; l’interopérabilité des vrais pairs reste à tester avant activation.
Les preuves locales ne démontrent ni autorité courante ni nouveauté scientifique.
