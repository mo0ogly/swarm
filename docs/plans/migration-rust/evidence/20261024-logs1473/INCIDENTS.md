# Replans1473 — originaux et décisions

Extrait exact du suivi de cet incrément ; les sorties originales restent dans
l’archive privée. Aucun échec antérieur n’est requalifié en succès.

Replan1473a : première recette arrêtée sur fr-etat-native-scripts-only ; le
serveur charge le nouvel adaptateur prévu, mais la liste de scripts du test
restait celle1472. Ajout explicite du seul asset attendu, pas de wildcard.
Original .cache/existing-cockpit1473-final et journey existing-cockpit-gayxiio2
conservés. Compilation dev40 réussie ; parcours Journaux encore non vérifié.
Propriétaire Codex ; vérifier d’abord ce parcours puis recette finale.

Replan1473b : fixture publique prepare/run refuse agent-execution-state-not-preserved
avant serveur (.cache/skynet-logs1473/target-first). Cause isolée par témoin pur :
GoRaw contenant <b> est échappé par encode_model puis ne compare plus égal à son
original littéral. Reçu final capture/provider facts produits via to_raw_value ;
la comparaison stricte de replace reste nécessaire. Options : assouplir GoRaw
global (écarté, contrat transversal), supprimer readback (écarté), normaliser
uniquement observations moteur avant garde (retenu). Helper persisted_raw et
3producteurs concernés ; aucune migration de données ni garde relâchée.
Témoin valide sans HTML et contre-exemple HTML conservés, fixture originale
intacte. Owner Codex ; correction à vérifier par même parcours public, bornes et
provenance inchangées. Aucun reçu de succès inventé pour l’exécution refusée.

Replan1473c : revue GLM bornée du correctif reçus tronquée (1600tokens tous en
raisonnement, zéro texte, 3830tokens déclarés), aucun avis valide ni acceptation.
Contexte réduit au helper pur et contrat codec, même budget de sortie/effort low ;
reprise non fondée sur une simple hausse. Revue SQL/exécution reste au principal
et aux tests publics ; ne pas attribuer de verdict au premier appel.

Revue GLM ciblée reçue (1240tokens déclarés) : compaction manquante du JSON brut
espacé. Le nouveau test unitaire reproduit ce défaut (1test exécuté, FAIL),
original unit.stdout/stderr conservé. compact appliqué hors chaînes avant capture
RawValue ; clés dupliquées et lexèmes restent conservés. Pas de modification de
PartialEq. La revue reste limitée au codec pur ; pas une acceptation indépendante
du moteur complet. Reprise : même test nommé puis parcours public complet.

Replan1473d : agent inert désormais completed, capture HTML sauvegardée ; API=CLI
pour3curseurs. Recette suivante refusée : hypothèse230sorties=230lignes de journal
fausse. Les sources séparent capture/transcript et4lignes cycle de vie. Original
target-corrected conservé ; témoin corrigé sur vraies lignes CLI et continuation
après première séquence, limite200 couverte par le test SQL permanent existant.
Ne fabriquer aucune ligne SQL. Lecteur HTML malveillant éprouvé par injection
client déclarée après témoin réel ; capture fournisseur reste un autre parcours.

Replan1473e : parcours HTTP et4combinaisons FR/EN/thème exécutés ; assertion
finale no-browser-errors refuse. Collecteur gardait détails en mémoire mais ne
les publiait qu’en succès : défaut de diagnostic de notre recette. Écriture
systématique des diagnostics en finally, originaux target-observed conservés.
Reprise ciblée pour obtenir cause exacte, aucune installation déclarée.

Replan1473f : diagnostic conservé :5HTTP500 provenant uniquement de mission
status, code mission-status-terminal-task-branch-unported (fixture publique agent
completed), distinct de1472-STATUS/provenance. Journaux ne consomme pas ce calcul.
Options : masquer ces erreurs dans la recette (écarté), porter maintenant toutes
branches terminales (chantier distinct), lire status à l’ouverture d’une vue qui
l’utilise (retenu). Journaux demande seulement contrôles/panneaux/liste agents ;
ouvrir une autre vue déclenche status avec refus intact. Pas d’autorité/fallback
inventé. Garde navigateur exigera absence de HTTP/console erreurs sur Journaux,
puis vérification séparée du refus au retour sur conduite. Owner Codex ; branche
terminale complète reste à porter avec ses vrais champs/fraicheur et fixtures.

Replan1473g : navigateur PASS après lecture paresseuse, refus terminal observé au
retour sur Conduite. Dernière assertion de pureté comparait tout agent show ; seul
supervision_heartbeat.age_ms change avec l’horloge. Original target-lazy conservé,
diff clés constaté. Témoin rectifié : octets SQL lus sans écriture sur fixture
propre + agent/desired/observed_status stables ; ne masquer aucune mutation.
La recette finale rejoue ce témoin exact.

Replan1473h — archivage : TarInfo.name était modifié avant la recherche du
fichier original dans la même archive, d’où KeyError. Sources et service intacts ;
publication finale non faite, fichier temporaire partiel conservé. Reprise du seul
stade archive : extraction par nom original puis copie de TarInfo pour renommer
la destination. Ne pas rejouer les mutations documentaires déjà terminées.
Critère : archive lisible, empreintes des24sources finales identiques. Owner Codex.
