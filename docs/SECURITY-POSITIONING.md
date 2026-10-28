# Positionnement de Swarm et références de sécurité

25 octobre 2026 — [English](en/SECURITY-POSITIONING.md).

## Positionnement produit

**Swarm est un moteur d’orchestration multi-agents à contrôle déterministe. Il
vise à relier chaque action à une identité, une autorité, un contenu approuvé et
un résultat vérifiable, avec des mécanismes de supervision et de reprise. Ses
garanties sont établies par parcours, sur une version et une configuration
identifiées.**

Le moteur commun organise objectifs, plans, tâches, dépendances, responsabilités,
contrôles et preuves. Les deux usages visés sont les projets de développement et
les processus métier ; leur séparation et leur maturité restent décrites dans
l’[architecture produit](PRODUCT-ARCHITECTURE.md). Le raisonnement des agents
propose des opérations ; le moteur vérifie les conditions des transitions qu’il
contrôle. Cette fonction ne démontre pas à elle seule le confinement des outils,
des processus ou des accès directs aux systèmes externes.

La propriété à rechercher est la continuité entre autorité, décision, engagement
et reprise : une autorisation doit rester liée à l’action exacte et à l’état
courant jusqu’au point où son effet peut être engagé. Rust et le déterminisme
facilitent certaines propriétés d’implémentation ; ils ne prouvent ni la justesse
des politiques, ni l’absence de défaut, ni la sécurité du déploiement complet.

## Rôle des références existantes

Les références ci-dessous n’ont pas toutes la même nature. Leur citation ne
démontre aucune conformité de Swarm. Cette cartographie est un positionnement
documentaire, sans sélection d’une baseline ni évaluation normative exhaustive.

| Référence | Nature et apport | Usage envisagé ou documenté dans Swarm |
| --- | --- | --- |
| [ISO/IEC 27001:2022](https://www.iso.org/standard/27001) et [42001:2023](https://www.iso.org/standard/42001) | Exigences de systèmes de management de la sécurité de l’information et de l’IA | Organiser responsabilités, risques, incidents et changements dans l’organisation exploitante ; aucune certification de Swarm revendiquée |
| [ISO/IEC 23894:2023](https://www.iso.org/standard/77304.html) | Recommandations de gestion des risques IA | Analyser les risques selon l’usage, les données et les conséquences ; aucune évaluation clause par clause réalisée ici |
| [OWASP Top 10 for Agentic Applications 2026](https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/) | Catalogue de risques agentiques | Sélectionner les menaces et les scénarios adversariaux ; un catalogue de risques ne constitue pas un protocole complet de qualification |
| [OWASP Agent Control Standard](https://genai.owasp.org/resource/agent-control-standard-acs/) et [dépôt ACS](https://github.com/GenAI-Security-Project/agent-control-standard) | Spécification de points d’interception et de contrôle à l’exécution | Étudier les correspondances de contrats, puis tester tout sous-ensemble de compatibilité ; aucune compatibilité ACS démontrée par cette page |
| [NIST SP 800-207](https://csrc.nist.gov/pubs/sp/800/207/final) | Architecture Zero Trust | Examiner la séparation entre décision de politique et application aux ressources ; aucune conformité globale déduite |
| [SPIFFE](https://spiffe.io/docs/latest/spiffe/concepts/) et son implémentation [SPIRE](https://spiffe.io/docs/latest/spire-about/spire-concepts/) | Identités vérifiables de workloads | Référence possible pour l’identité des processus ; aucune intégration SPIFFE/SPIRE annoncée. Authentifier un workload ne définit pas ses droits métier |
| OIDC et OAuth | Protocoles et mécanismes d’authentification et d’autorisation | Le [contrat cible](architecture/ACTIONS-CONFIANCE.md#identité-mandats-et-approbations) décrit les choix historiques ; le noyau Rust actuel utilise des identités locales. Aucune fédération réelle requalifiée par cette page |
| [OWASP ASVS 5.0.0](https://asvs.owasp.org/) | Exigences de vérification des applications et services web | Référence proposée pour vérifier API, sessions, permissions et configuration ; aucun niveau ASVS évalué ici |
| [NIST SP 800-53 Rev. 5](https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final) | Catalogue de contrôles de sécurité et de confidentialité | Douze contrôles sélectionnés sont déjà reliés à des preuves contextuelles dans la [matrice R19](actions-confiance-referentiels.md), sans score ni conformité globale |
| Biba | Modèle de contrôle des flux d’intégrité | Moniteur I0–I3 dans le [noyau Rust des actions de confiance](actions-confiance-rust.md) ; ni preuve de vérité des contenus, ni protection sémantique générale contre l’injection, ni contrôle d’accès Linux |

La page OWASP d’ACS porte la date du 1er septembre 2026. Le README du dépôt
consulté décrit déjà une couche déterministe avec un recours optionnel à un
LLM. Swarm doit donc préciser son apport par ses contrats et ses preuves plutôt
que revendiquer l’invention de cette séparation. Une future comparaison devra
figer la révision ACS évaluée, les exigences applicables et les essais exécutés.

Le NIST a annoncé son [AI Agent Standards Initiative le 17 février 2026](https://www.nist.gov/news-events/news/2026/02/announcing-ai-agent-standards-initiative-interoperable-and-secure).
Son [point du 29 septembre sur l’identité et l’autorisation](https://www.nist.gov/news-events/news/2026/09/comments-software-and-agentic-ai-identity-concept-paper)
présente des retours et un premier cas d’implémentation DevSecOps ; cela ne
constitue pas une norme finale de certification agentique. Le Top 10 « 2026 »
est, lui, daté du 9 décembre 2025 sur la page OWASP : millésime et publication
sont distincts.

## Frontières de confiance à démontrer

| Fonction | Responsabilité | Preuve attendue |
| --- | --- | --- |
| Gardien des règles | Vérifier identité, mandat, politique courante, contenu exact et validations requises | Une décision positive n’est consommable que pour l’opération et l’état autorisés ; une analyse IA ne crée pas de droit |
| Gardien des clés | Conserver les secrets et délivrer les capacités limitées au consommateur autorisé | L’agent ne peut ni lire les secrets critiques ni obtenir une capacité plus large par une autre voie |
| Garde-barrières | Appliquer le contrôle au niveau des connecteurs, processus, fichiers et accès réseau concernés | Les chemins directs, descendants et accès alternatifs inventoriés ne permettent pas de contourner le refus |

Ce sont des fonctions d’architecture. Trois modules ayant les mêmes privilèges
et pouvant modifier mutuellement leurs politiques ou leurs clés ne démontrent
pas trois frontières indépendantes. Un journal ou une observation eBPF après
l’effet ne remplace pas une barrière préventive. La démonstration doit déclarer
son modèle de menace, notamment les limites face au même UID ou à un hôte
privilégié compromis.

## État documenté au 25 octobre 2026

| Périmètre | État consigné | Limite de la revendication |
| --- | --- | --- |
| Actions de confiance Biba Rust | Le [plan BIBA-RUST](plans/migration-rust/PLAN.md#livré-localement--biba-rust-g7-r09t2-25-octobre-paris) consigne la livraison locale du moniteur, des mandats, accords exacts, effets simulés et reprises, avec CLI/API/page et Admin FR/EN | Périmètre `trust-actions` ; aucun paiement, droit externe ou confinement OS réel. Les preuves consignées ne sont pas rejouées par cette analyse documentaire |
| Identité et autorité des agents | [Contrats et composants internes](architecture/AGENT-IDENTITY-CONTRACT.md), avec limites de raccord explicites | Ne prouve pas l’authentification et l’autorisation de tous les lancements, usages mémoire et outils |
| Mémoire | [Versions, sources et conditions d’usage](PROJECT-MEMORY.md) ; [reprise Rust FPA02A](plans/migration-rust/FPA02A-MEMOIRE-20261025.md) de références mémoire historiques | Le lot de références ne clôt pas la mémoire permanente versionnée ni tous les droits de lire, résumer et transmettre. Un retrait ne supprime pas les contextes déjà reçus |
| Confinement général et médiation des secrets | [AC-02 enregistré, non démarré](plans/actions-confiance/securite-execution-apex.md) | La référence OpenShell ou une politique YAML ne prouve aucune protection AC-02 active ; compatibilité et application restent à démontrer |
| Correspondance de contrôles | [R19 contextualisée](actions-confiance-referentiels.md) : neuf partiels, deux non évalués, un hors périmètre | Sous-ensemble illustratif, sans baseline sélectionnée, certification, conformité globale ni évaluation indépendante |

## Proposition de profil d’assurance de l’exécution agentique

**PAEA — Profil d’Assurance de l’Exécution Agentique** est le nom proposé par
l’auteur dans son [article](communication/linkedin-swarm-securite-20261025/article-fr.md).
Il désigne ce cadre de travail documentaire. Les identifiants de suivi PA-01–08
ci-dessous sont conservés ; ce nom ne crée ni nouveau mécanisme du moteur, ni
label ou niveau de certification.

Le périmètre proposé couvre les actions dont une erreur, un défaut d’autorité,
une répétition ou un contrôle insuffisant pourrait avoir des conséquences graves
pour les personnes, les droits, les finances, la confidentialité, la sûreté ou la
continuité d’un service essentiel. La criticité se juge aussi sur la trajectoire
et les effets cumulés, pas seulement sur une opération isolée. Les niveaux
d’assurance et leur protocole de qualification restent à spécifier.

Ce profil est une **proposition documentaire**, pas un nouveau standard adopté,
une politique active, une qualification réalisée ou une autorisation de démarrer
AC-02. Il prolonge la matrice existante au lieu de la remplacer. Les identifiants
PA ci-dessous servent au suivi ; les correspondances externes restent à évaluer.

| Exigence proposée | Critère observable de qualification | Point de départ et propriétaire de la suite |
| --- | --- | --- |
| PA-01 Identité et délégation | Chaque action remonte à un principal et un mandat courants ; une délégation ne dépasse pas les droits délégables du parent | Contrat d’identité ; responsable du raccord identité puis intégrateur du déploiement |
| PA-02 Intégrité et admission | Des données externes ou un champ client ne créent pas de droits ; toute admission crée un objet attribué lié à l’original | Noyau Biba pour son périmètre ; responsable Trust et futurs consommateurs |
| PA-03 Mémoire gouvernée | Sources, versions et droits d’usage sont conservés ; un retrait bloque les nouveaux usages visés sans réécrire l’historique reçu | Contrats mémoire ; responsable G8 et intégrateur identité/mémoire |
| PA-04 Contrôle au point d’engagement | Une modification du contenu approuvé ou une révocation antérieure au point de sérialisation empêche l’effet ; une panne du contrôle refuse l’action sensible | Actions simulées et contrats d’effet ; propriétaire du service d’effet et de sa politique |
| PA-05 Capacités et voies d’accès | L’agent ne récupère pas de secret critique et ne contourne pas le refus par outil, fichier, descendant ou réseau | AC-02 ; responsable du confinement et opérateur de l’environnement, après ses préconditions de démarrage |
| PA-06 Effets et reprise | Une réponse perdue conserve une issue connue ou inconnue ; réconciliation et idempotence empêchent un second effet pour la même intention | Deux connecteurs simulés ; propriétaire de chaque connecteur réel avant son automatisation |
| PA-07 Validation humaine | Quand la politique l’exige, un avis IA ne remplace pas la décision humaine authentifiée liée à l’opération exacte ; séparation des rôles vérifiée lorsqu’exigée | Contrats de validation ; responsable identité/approbation et responsable métier |
| PA-08 Preuves et qualification | Chaque décision et effet observé sont reliés aux entrées, révisions et originaux ; altération et omission sont détectées sous les hypothèses déclarées | Recettes et preuves locales ; responsable de qualification puis évaluateur indépendant mandaté |

Une fiche d’évaluation doit déclarer version et empreinte du binaire, configuration,
modèle de menace, périmètre, exigence externe/version, applicabilité, mécanisme,
test nommé, témoin valide, refus, reprise, preuve, limite, propriétaire et état.
Une unité réussie, une simulation, une intégration réelle et une évaluation
indépendante conservent des statuts séparés. Un pourcentage moyen ne compense
pas l’échec d’un invariant requis.

La première séquence proposée est de figer le périmètre et les références, relier
les acquis vérifiables à R19, puis traiter les raccords et les protections
manquantes dans leurs plans autorisés. Chaque nouvelle intégration doit exercer
le contenu modifié après accord, la révocation concurrente, la réponse perdue,
la panne du contrôle et les chemins de contournement applicables. L’arrêt ou la
révocation bloque les nouvelles opérations concernées sans prétendre annuler
un effet déjà engagé. L’absence de preuve laisse le critère ouvert.

## Vocabulaire de communication

| Formulation | Justification nécessaire |
| --- | --- |
| Inspiré de | Référence et principe repris explicités |
| Correspondance documentée | Exigences sélectionnées, contexte, écarts et preuves reliés |
| Compatible avec un sous-ensemble | Révision fixée, périmètre publié et essais de compatibilité exécutés |
| Exigences satisfaites sur un périmètre évalué | Critères applicables, configuration et preuves actuelles, échecs et limites déclarés |
| Certifié | Certification effective, organisme, périmètre et validité vérifiables ; aucune revendiquée pour Swarm ici |

Une signature ne prouve pas la vérité d’une mémoire. Biba ne garantit pas la
confidentialité, et une identité authentifiée ne vaut pas autorisation métier.
Le terme « opposable » décrit ici une contrainte technique démontrée dans son
périmètre, sans présumer d’une valeur juridique des preuves.

Pour les usages critiques, la position proposée est de conditionner l’engagement
autonome aux garanties requises par le contexte. Ce choix d’ingénierie ne décrit
pas une interdiction réglementaire générale. L’évaluation réglementaire et
sectorielle du déploiement appartient à son responsable mandaté.

Le [dossier LinkedIn et le Word](communication/linkedin-swarm-securite-20261025/README.md)
conservent la dernière source auteur et une fusion explicitement demandée avec
les compléments de positionnement. Chaque paragraphe de cette source est conservé
dans l’ordre ; les ajouts sont tracés séparément. Les anciennes versions restent
historiques. Cette documentation ne lance aucun
chantier, ne modifie aucune politique d’exécution et ne requalifie aucun runtime.
