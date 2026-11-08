# Étude — contrôler les actions face à l’injection indirecte

8 novembre 2026. Étude documentaire demandée par Fabrice ; aucune exécution
de scénario, modification moteur, délégation, installation ou publication.

## Source et portée

[Article de Peter Jonathan Wilcheck et Ray Anderson](https://techonlinenews.com/ai-agent-cybersecurity-prompt-injection-control-actions/),
« When AI Agents Become the Attack Surface: Security Must Control the Action »,
daté du 8 novembre 2026. Lecture directe du HTML après refus du lecteur web ;
titre, signature, texte et liens de référence lus. Ce constat ne prouve pas
les capacités des produits cités.

Thèse retenue : un contenu externe peut influencer une action ; il faut vérifier
la frontière d’autorité au point d’effet, la couverture des contrôles et la reprise.
L’article recommande aussi une approbation compréhensible et une réconciliation
des effets après interruption. Ces recommandations ne constituent pas des
résultats expérimentaux sur Swarm.

Sources primaires complémentaires effectivement lues :

- [OWASP LLM01:2025](https://genai.owasp.org/llmrisk/llm01-prompt-injection/),
  sections injections indirectes et stratégies : privilèges minimaux, contrôle
  dans le code, approbation des opérations privilégiées ; aucune défense parfaite.
- [Anthropic, Trustworthy agents in practice](https://www.anthropic.com/research/trustworthy-agents),
  introduction et « How agents work » : modèle, harness, outils et environnement
  doivent être considérés ensemble. Description du fournisseur, pas qualification.
- [Microsoft Learn, runtime protection, Preview](https://learn.microsoft.com/en-us/defender-endpoint/ai-agent-runtime-protection-overview),
  « How it works » : points d’inspection dépendant des interfaces natives et flux
  couverts ; réseau sans certificate pinning ni HTTP/3. Ne démontre aucune
  intégration Swarm. Les annonces Microsoft du 5 novembre et le document NIST
  cités par l’article ne sont pas vérifiés dans cette étude ; aucune capacité,
  maturité ou statistique n’en est déduite.

## Rapprochement avec Swarm — code lu, pas effet courant observé

Checkout `main`, HEAD `4d2c4f96b5aede2193ca62c72e7597f9ad5b4296`, avec
documentation déjà modifiée avant cette étude. Les passages ci-dessous ont été
lus avant rédaction ; aucun PASS historique n’est transféré au checkout courant.

| Proposition exacte | Passage lu et portée | Limite |
| --- | --- | --- |
| Un scénario R1 existe déjà pour un faux accord dans une sortie d’outil. | `rust/crates/swarm-engine/tests/trust_biba.rs`, `biba_tool_result_cannot_authorize_effect` : trois formes de contenu, témoins, refus, état métier, reprise, réouverture et rejeu. | Lecture du test ; pas de nouveau lancement, modèle réel, humain enrôlé ou médiation des outils natifs. Le rapport R1/R2 rapporte son exécution antérieure. |
| Le chemin Trust lu vérifie davantage qu’un texte déclarant un accord. | `rust/crates/swarm-engine/src/trust/operations.rs`, branche `approve` : rôle, Biba, séparation configurée, ressource et termes exacts, mandat courant ; engagement : mandat actif, état approuvé, preuves d’accord. | Parcours Trust et effets simulés lus ; aucune couverture universelle des sorties réseau/shell. |
| Biba n’authentifie pas à lui seul les niveaux ou leur provenance. | `rust/crates/swarm-engine/src/biba.rs`, contrat de module et `check_biba` : relations lecture/écriture/invocation ; responsabilités explicitement laissées aux appelants. | Ne prouve pas confinement OS, provenance des données ou contrôle sémantique du modèle. |
| Une correspondance d’empreintes n’est pas une autorisation. | `rust/crates/swarm-protection-contract/src/lib.rs` et `approval.rs::check_approval_documents` : contrats purs, rapport `offline-data-only`, authenticité/validité/effet non établis. | Ne pas proposer de brancher un JSON d’accord comme autorité. |
| Un garde natif spécifique n’est pas un confinement général. | `rust/crates/swarm-engine/src/root_store/task_validation/process.rs`, contrat de module, `source`, `gate_command` : politique enregistrée, identité de processus, source et répertoire contrôlés. | Transport des contrôles de tâche ; ni inventaire exhaustif des outils ni confinement contre un écrivain hostile de même UID. |
| La liaison tâche/tentative a déjà un cadrage distinct. | `POC06-TASK-MANDATE.md`, points 6–12 et 15 : PoC isolé, contexte fourni par l’hôte, raccordement natif à qualifier. | Le PoC ne prouve pas ce raccordement. Pas de nouvelle délégation autorisée ici. |

Les chemins Rust sont relatifs à la racine du dépôt. État historique et limites :
[rapport R1/R2](PETER-WILCHECK-RAPPORT-R1-R2.md). La phrase historique disant
que l’article n’avait pas été audité reste vraie pour ce travail antérieur ;
la présente lecture est une preuve nouvelle, pas une réécriture de son historique.

## Options et décision proposée

Option A : enrichir d’abord prompts et détection de formulations suspectes.
Cela peut être étudié, mais ne vérifie pas le blocage d’une action quand le modèle
suit quand même l’instruction. Aucun détecteur nouveau proposé comme autorité.

Option B recommandée : qualifier d’abord un chemin réel jusqu’à son effet,
en réutilisant R1 et les scénarios existants. Le moteur reste propriétaire des
contrôles ; l’autorité métier reste extérieure au contenu analysé. C’est une
proposition d’ordre de travail, pas une décision d’implémentation adoptée.

| Suite | Travail proposé et réemploi | Critère observable | État / propriétaire |
| --- | --- | --- | --- |
| TN-01 | Inventorier un parcours disponible : origine externe → contexte réellement reçu → appel outil → contrôle → cible ; inventorier les autres chemins vers cette même cible. Réemploi du complément transversal POC. | Chaque chemin a son point de contrôle, son autorité, son observateur d’effet et ses exclusions explicites ; une intégration absente reste un prérequis. | PROPOSÉ ; Codex pour qualification. |
| TN-02 | Porter le cas R1 sur le parcours choisi : l’acteur sans droit tente l’action suggérée par le contenu, même si le modèle serait docile. Séparer ensuite une éventuelle évaluation avec modèle réel. | Témoin autorisé réussi avant chaque faute ; refus avant effet, cible inchangée observée, diagnostic sûr ; reprise exacte autorisée. | NOT TESTED ; Codex, après autorisation d’exécution. |
| TN-03 | Réutiliser POC-01/02/06 : substitution destination/contenu, changement pertinent de politique ou contexte tâche/tentative après accord. | L’ancien accord ne suffit pas ; refus avant effet puis reprise avec autorité applicable ; historique conservé. | NOT TESTED ; Codex, autorité compétente pour décision. |
| TN-04 | Réutiliser POC-03/04 : révocation, effet engagé mais réponse perdue, réouverture/rejeu. | Nouvelles actions refusées selon révocation ; effet engagé conservé comme connu/inconnu puis réconcilié ; pas de duplication injustifiée. | NOT TESTED ; Codex et propriétaire du connecteur. |
| TN-05 | Pour une opération nécessitant consentement, rapprocher données, destination et conséquences affichées des paramètres réellement engagés. | Accord lié aux termes exécutés ; mutation pertinente impose une décision applicable ; cas FR/EN et états d’erreur si UI touchée. | PROPOSÉ ; Codex, Fabrice pour arbitrage produit. |

Ne pas lancer tous les PoC : leur report antérieur reste applicable. Commencer
par TN-01 dans un incrément ultérieur autorisé, puis choisir un seul parcours
testable. Aucun développement n’est autorisé implicitement par cette étude.
Le choix d’un export documentaire serait un scénario utile, **pas une capacité
Swarm établie ici** ; sélectionner le connecteur après lecture de ses producteurs.

## Conservation, revue et reprise

Découvertes conservées : TN-01 évite une couverture supposée depuis un journal
de refus ; TN-02 distingue test du moniteur et comportement du modèle ; TN-04
évite d’assimiler interruption à annulation d’effet. Valeur attendue, origine,
propriétaires et critères figurent ci-dessus. Leur effet reste non vérifié.

Prochaine condition de reprise : autorisation de qualifier/exécuter le parcours
retenu, avec fixture isolée, données synthétiques et recette existante adaptée.
Aucun appel payant, mission active ou secret réel nécessaire à cette proposition.

Revue de l’incrément : uniquement étude et liens de suivi FR/EN ; aucune fonction,
politique, constante, valeur par défaut ou issue d’erreur produit modifiée.
Pas de docstring, build, version livrée ou redémarrage applicable. Pas de tests
produit pour cette étude. Les fichiers persistants transmettent le cadrage ;
aucun contexte embarqué ou processus en cours n’est mis à jour. La capture
volumineuse de l’article n’est pas recopiée ; seuls liens, lecture et portée sont
conservés. Les autres modifications présentes sont préservées.

## English handoff

### Empreintes des sources lues

SHA256 des six sources Rust rapprochées ci-dessus, relevées pendant l’étude :

```text
364182966d013cc2422e5fa99658fbf49365191b857d903d355c054bbdfd25f0  trust_biba.rs
9ec3ca6f30c5344ef52d65c27061ad058a54317a5054d9de508c82881e87197b  trust/operations.rs
bd923188264230c9ac1219a520753143278ece7d0117736b702046afa784352c  biba.rs
578ac710648b6846eb0ffcae52144435587c9d1c96d096debedf40345a194a4a  swarm-protection-contract/src/lib.rs
61d2cbb4fc93a34c6be800c09321a8edc67e4852be15ba312e1b91799fa90267  swarm-protection-contract/src/approval.rs
decd57877c0481bbf6643ae4451c3cafa89ada37afd26b7149930bc00baf8162  root_store/task_validation/process.rs
```

Ces empreintes identifient les octets lus, pas un build ni leur effet à l’exécution.

Documentary study only. The article recommends controlling consequential actions
independently of model compliance. Current source inspection finds an existing
Trust forged-approval fixture, a scoped Biba guard and pure offline approval-data
checks; none establishes universal native-tool mediation or OS containment.
Historical R1 results were not rerun or transferred to current HEAD. Prefer one
end-to-end action-path qualification, reusing existing PoCs, over a new parallel
guardrail system. TN-01–05 remain proposed/not tested; prior execution deferrals
remain effective. No runtime, policy, model call or deployment changed.
