# Brouillon — section 7 : attribution des fautes (QR3), protocole pré-enregistré

Statut : protocole rédigé le 5 octobre 2026, **avant** la constitution et la lecture du
corpus. Toute modification ultérieure de la taxonomie, de la table de correspondance ou des
mesures sera datée et signalée comme écart au protocole dans l'article.

---

## 7. Attribution des fautes

### 7.1 Question

QR3 : quand une tentative échoue ou qu'une tâche se bloque, le moteur attribue-t-il la faute
au bon responsable : l'agent, l'environnement, la règle, ou le moteur lui-même ? Une
attribution fausse a un coût direct sur un flux financier : une indisponibilité prise pour une
faute de l'agent déclenche une régénération du livrable (défaut D3) au lieu d'une attente.

### 7.2 Taxonomie

Cinq classes, dérivées de MAST (Cemri et al., 2025), qui classe les défaillances des agents mais
n'a aucune catégorie pour l'orchestrateur lui-même :

| Classe | Définition |
|---|---|
| Agent | L'agent a produit un livrable ou un comportement fautif (candidat modifié, rapport ancien relayé, sortie invalide). |
| Environnement | Une cause extérieure à l'agent et au moteur : service indisponible, processus tué, réponse perdue, écriture concurrente d'un tiers. |
| Règle | Une règle juste mais mal calibrée a arrêté un travail correct (budget trop bas, délai trop court). |
| Moteur | Un défaut du moteur a causé ou aggravé l'échec. |
| Inconnu | Les traces ne permettent pas de trancher. Classe assumée, jamais utilisée par défaut. |

Une unité peut recevoir une classe principale et, au plus, une classe aggravante (exemple :
environnement, aggravée par le moteur).

À part la cause, chaque unité reçoit un résultat : arrêt correct (la faute n'a produit aucun
effet faux) ou non. *Amendement du 6 octobre 2026 :* ce résultat remplace la classe « faute
injectée correctement arrêtée » du § 6.9, qui mêlait cause et résultat.

### 7.3 Corpus

- **Source.** Un lot S séparé de la campagne principale, dont les racines sont conservées
  (la campagne principale supprime celles des exécutions OK) : clé métier, les dix cas
  (aucune faute, F1 à F8, F4e), 10 graines (1000 à 1009), soit 100 exécutions.
- **Unité d'annotation.** Chaque tentative terminée autrement que `completed` avec validation
  acceptée, chaque tâche passée `blocked`, chaque `planning.failure`. Les exécutions sans
  aucune de ces unités ne contribuent pas au corpus.
- **Pièces fournies aux annotateurs.** Pour chaque unité : journal de la tentative, diagnostic
  de tentative et motif de blocage enregistrés par le moteur, événements adressés au
  responsable, contrôles exécutés et leurs codes de sortie. Le nom de la faute injectée, la
  graine et la condition sont retirés.
- **Limite de l'insu.** Certaines traces révèlent la faute (un 503 dans la sortie d'un
  contrôle, un code 137). L'insu est donc partiel ; nous le signalons sans le corriger.

### 7.4 Vérité de référence

Contrairement à MAST, la cause est connue par construction : chaque faute est injectée. La
vérité de référence de chaque unité est fixée **avant** l'annotation, par la table suivante,
puis corrigée unité par unité si les traces montrent une cause différente (ces corrections
sont publiées).

| Faute injectée | Classe principale attendue | Remarque |
|---|---|---|
| F1 réponse perdue | Environnement | |
| F2 lanceurs concurrents | Environnement | écriture concurrente d'un tiers |
| F3 arrêt brutal (137) | Environnement | |
| F4, F4e candidat modifié | Agent | |
| F5 service indisponible | Environnement | aggravée par le moteur si la tâche est régénérée (D3) |
| F6 budget épuisé | Règle | budget volontairement trop bas |
| F7 propriétaire expiré | Environnement | |
| F8 rapport ancien relayé | Agent | |
| tout cas, `planning.failure` sur SQLITE_BUSY | Moteur | défaut D5 |
| aucune faute | sans objet | toute unité produite est examinée comme faux positif |

### 7.5 Diagnostic automatique comparé

Le moteur classe chaque échec de reprise dans une catégorie (`recovery.go`) : `transient`,
`environment`, `conflict`, `business`, `unknown`. Correspondance fixée d'avance :

| Catégorie du moteur | Classe |
|---|---|
| `transient`, `environment`, `conflict` | Environnement |
| `business` | Agent |
| `unknown` | Inconnu |

Le moteur n'a pas de catégorie pour la règle ni pour lui-même. Son rappel sur ces deux classes
est donc nul par construction ; nous le rapportons comme un constat, au même titre que
l'absence de catégorie orchestrateur dans MAST.

### 7.6 Annotation

*Amendement du 6 octobre 2026, avant constitution du corpus : aucun second annotateur humain
n'est disponible.*

- **Annotateur humain unique** : l'auteur étiquette toutes les unités avec la taxonomie de 7.2,
  après une séance d'étalonnage sur dix unités tirées hors corpus, exclues des mesures.
  L'accord entre annotateurs humains n'est pas mesuré ; l'article le déclare. Si un second
  annotateur humain devient disponible avant l'analyse, il étiquette le même corpus selon la
  même procédure et la mesure 1 est rétablie.
- **Annotateur LLM, mesure secondaire** : un modèle différent de celui des agents mesurés
  (`claude-opus-5-5`, contre `claude-sonnet-5` pour les agents du lot réel), sans outils, reçoit
  pour chaque unité les mêmes pièces que l'annotateur humain (7.3) et la même consigne écrite
  (définitions de 7.2, une classe principale, au plus une classe aggravante, et « inconnu »
  autorisé). Consigne, modèle, version du client et réponses brutes sont publiés. Il étiquette
  indépendamment : ni l'auteur ne voit ses étiquettes avant d'avoir fini, ni le modèle ne voit
  celles de l'auteur.
- Le modèle n'est pas un second annotateur humain : son accord avec l'auteur est rapporté comme
  une mesure distincte (sur le modèle de MAST, qui rapporte séparément l'accord humain et celui
  d'un juge LLM), jamais comme κ inter-annotateurs.
- La vérité par construction (7.4) reste la référence principale ; aucune étiquette n'est
  corrigée après comparaison.

### 7.7 Mesures

1. Accord entre annotateurs humains : κ de Cohen sur la classe principale, avec intervalle à
   95 % par bootstrap (1 000 rééchantillonnages, graine fixée). *Non mesuré tant qu'un seul
   annotateur humain est disponible.*
2. Diagnostic du moteur contre la vérité de référence : précision et rappel par classe,
   matrice de confusion, taux d'`unknown`.
3. Auteur contre la vérité de référence, et annotateur LLM contre la vérité de référence :
   exactitude, séparément, pour vérifier que la taxonomie est applicable à partir des seules
   traces.
4. Accord auteur et annotateur LLM : κ de Cohen avec le même intervalle, rapporté comme accord
   humain–modèle, et liste des désaccords.
5. Cas où le moteur est fautif ou aggravant : nombre, défaut en cause (D1 à D5), et part de
   ces cas que le diagnostic du moteur attribue à l'agent.

Aucune mesure n'est agrégée en score unique. Avec environ une centaine d'unités, les
intervalles seront larges ; QR3 est une étude exploratoire, pas un test d'hypothèse.

---

## Points à décider avant le lot

1. **Second annotateur humain** : aucun à ce jour (amendement 7.6) ; place conservée.
2. **Lieu du diagnostic dans `state.db`.** Vérifier sur une racine conservée où se trouvent la
   catégorie de reprise et le motif de blocage, puis écrire l'extraction des pièces de 7.3.
3. **Lancement du lot** : après la campagne principale, pour ne pas fausser ses durées.
