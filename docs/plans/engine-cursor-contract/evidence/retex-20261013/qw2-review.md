# Revue indépendante RETEX-QW2 et frontière QW1

Base b61694264cbeaf0d72cea8e55201c1fd1f7aaed1, delta non commité figé pendant recette navigateur. SHA du diff suivi examiné eb011490782d0604343b1f21edfa727ecb25f02391881f9bee7dee48c31c7080 ; fichiers nouveaux liés individuellement ci-dessous. Lecture seule, aucun autre agent, test/build/provider concurrent, accès base de mission privée ou modification candidat.

**Verdict : request changes. Un finding P2 ouvert : registre worker silencieusement limité aux 200 dernières tentatives. Aucun autre défaut concret identifié dans le périmètre examiné.**

## Finding P2 — conserver l’historique complet du registre

Ligne primaire du nouveau delta : internal/engine/mission_insights.go:580, boucle575–580. Le nouveau registre ajoute seulement les agents passés par missionStatus, lequel les charge via s.agents(work) à mission_status.go:348. Ce producteur impose ORDER BY rowid DESC LIMIT200 à agents_store.go:273. planning_calls est lu sans ce cap ; le registre devient donc silencieusement incomplet seulement pour les tentatives worker. La UI expose leur nombre sans total/complete ou avertissement de tranche incomplète, alors que le plan QW2 exige chaque réservation/tentative.

Déclencheur déterministe : un Work avec201 lignes agents enregistrées, chacune attribuée au Work et avec usage connu, puis lecture publique mission spending. Le nouveau calls contient200 lignes agent, exclut la première tentative et sa consommation, tandis que les réservations de planification restent toutes présentes. La mise en œuvre des autres cartes avait déjà ce plafond ; le défaut est sa réutilisation pour le nouveau contrat de registre historique complet. Je n’ai pas exécuté cette fixture pendant le navigateur ; le chemin source et le SQL établissent le contre-exemple. Faire son rouge public isolé avant correction, puis vert.

Correction générale conseillée : lecture spécifique complète, scopée par Work, pour le registre et ses agrégats, ou pagination publique avec total/complete si choisie explicitement. Ne pas enlever globalement le cap des autres consommateurs. La régression doit couvrir plus de200 agents, première tentative avec usage distinct, étrangères exclues, absence/refus conservés et pas de double compte.

## Contrats correctement préservés

- Agent.WorkID étranger explicite filtré, planning_calls paramétré par w.ID, réservations released exclues. Le producteur public agents(work) est lui-même scopé SQL. Les agents historiques sans WorkID dans le body restent admis sur ce chemin scopé, pas une requête inter-mission. Les identités agent/ et planning/ évitent la collision de namespaces.
- Attribution worker depuis Agent persisté : Task/Attempt/Provider/Scope depuis tâche propriétaire, requested_model depuis ModelRoute et observed_model depuis ReportedModel. Aucun modèle ou fournisseur historique planner/reviewer n’est inventé à partir de la configuration courante.
- Usage nul absent et JSON null avec whitespace restent nil. Objet partiel, type invalide, jetons/cache/coût négatifs retournent une erreur ; pas de passage à0/0 ou succès vide. Les pointeurs facultatifs cache et coût conservent absence et zéro explicite distincts. Aucun cache additionné aux input/output et aucun tarif appliqué en remplacement d’un coût non rapporté.
- UI utilise node/text et jamais HTML pour ces valeurs ; lazy details, rôles, états, provider/model inconnu, absence usage/coût et zéro explicite correctement distingués. Les variantes FR/EN nouvelles sont synchronisées locales/en.json/i18n-en.js. Le registre explicite qu’une réservation/tentative n’est pas une preuve d’inférence envoyée.

## Preuves existantes et limites de verification

Journal Go propriétaire retex-qw2-call-ledger-final-green.jsonl SHA fd6f2703cb2073de6801a31363df949248298985b24f6bc0501f873816bd47ba : trois tests top-level réellement PASS (SeparatesUnitsAndMissingCost, CountsManagedAndAutomaticControlEvents, CallLedgerAttributesMissingUsageAndCaches). Commande -race rapportée par propriétaire. Original null-whitespace rouge SHA77be85edc25e33ff4ea524731feee7760ae61a44817b9d0237e17551fb13e55f : JSON null became reported zeros, FAIL valide conservé. Lecture du test Node couvre FR/EN, null/zero/cache, texte malveillant et réouverture sans doublon ; son exécution n’est pas répétée par reviewer. Recette browser en cours au moment de la revue, aucune qualification visuelle/PASS inventée.

## QW1 : aucune acceptation ou coût fabriqué identifié

Le harnais étiquette explicitement le premier producteur comme double déterministe fautif, sans appel modèle ; son delivery positif volontairement faux est une entrée non fiable, pas un avis ou contrôle moteur inventé. Le contrôle structural retourne réellement succès car0 est un entier ; son insuffisance à prouver42 est expliquée. La revue doit/refuse réellement le défaut sémantique avant acceptation. Le retour correct exige ensuite le bundle et assertion exacte42.

Lecture des reçus publics QW1 : result.json SHA2fbe0dbe8bc0e2bdd5c8327f6c703f2dbf6d43093a7bebee36971529937d227d, fault-injection.json SHA7a50decd5e3ba0a605c1226aa8c8b855bf78dacae39a79fa62dfd303fb7e0c0e, final-public-work.json SHA362ecfedbe21359ce94bead3e9c5e581d21f6d89be714527c3b3bf836f451da2. Public r22 validated1/stale0 ; tentative initiale et corrective distinctes, PreviousReviews conserve changes_requested sur91f57dc, avis final passed sur8f44756859bb6c6b6d30c796d25d8d0635cc5c5d ; événement planning.decide attribué avec input review et operation retry pour même task, instruction neuve imposant42 et test discriminant. Judge end cycle1. result rapporte assertion bundle exacte exit0 et0 intervention après start ; ce dernier compteur hardcodé seul n’est pas une preuve exhaustive de toute intervention extérieure, mais le harnais inspecté ne décide ni retry/review/accept après start. Les opérations de préconfiguration/double et arrêt après fin restent explicites.

Spending QW1 SHA18c6a1f9077b6f71a9409f417bfd941da5fc48e102eedf548697c21ed2080793 conserve manque d’usage du double et coûts absents pour tous rôles (attempts_with_cost0, without_cost positif). Le champ agrégé reported_usd0 n’est pas présenté comme facture gratuite. Pas de coût réel inféré. Pas de nouvel appel provider ou test fonctionnel bundle effectué par reviewer ; ceci est une revue des sources et preuves publiques reçues, pas répétition de campagne.

## Empreintes sources nouvelles et affectées

internal/engine/spending_calls.go : 17d274b7795388da08bed63add537b6ad6ca2a8fe4e7c47c8796cbdc25ea97cf

internal/engine/spending_calls_test.go : 7799af855e14ca41586adc19a29d2fc33e61010019971b4667ebc127917e502a

internal/engine/mission_insights.go : ca6c4f1b8f73843e04f45366eaf7ec91244bc43fa83cb928dfa823d58943d94a

web/mission-insights.js : d291a341b41a7292efab5b93457282bf8ee282f07fc978f39c99ae5ea7cf306f

locales/en.json : d0b8e7d544846a8dd98dff539b0813b2a75f2b1f7479baddcda6e20d14d8fa17

web/i18n-en.js : be047dcb55a7c7397a536970b2ce42c2ea5d76a099c10de2c5a3f1882d7a935f

tests/call_ledger_test.cjs : f4e2d24c9c8ab8d4b341ce33f01dc9d6817b5079a64b34aa466b1421f6c04b5b

tests/call_ledger_ui.cjs : 497e458752dfe4c78dce077be8732c65db323b7dac4a1b7a77bd79381da96441

tests/retex_real_campaign.py : 8127e2e6b16ae222ab5cd4f505b030d8777adf38bec563b77259e8c4c31e6700

docs/plans/engine-cursor-contract/suites-retex-20261013.md : 706900e1d0eed3a7002c95519fdefcc68e01378b8def4b23e3959e67ca708c40

Sources libérées, aucun check reviewer actif. Condition de qualification QW2 : corriger le registre tronqué avec rouge/vert ciblé, puis examiner le delta et la preuve browser/publique réelle du candidat corrigé. Ne pas refaire QW1 payant sans autre dépendance modifiée.
