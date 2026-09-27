# Suite RD3 — dossier de revue et capacité

## Mesures disponibles, sans nouvel appel IA

Le prévol installé indique 1 101 354 octets de contenu encore à inspecter et deux
appels finaux, pour sept appels disponibles. Même en supposant un transport sans
aucune consigne, empreinte ni enveloppe, cinq inspections à 196 608 octets ne
transporteraient que 983 040 octets. Il faut donc éliminer **au moins 118 314 octets
de répétitions exactes** pour seulement envisager cinq inspections. Le gain réel
nécessaire est supérieur, puisque les consignes et schémas occupent de la place.

Un inventaire diagnostique du journal historique à sept entrées
`fragment-journal-5d2856e9c81a678510de45623550d70ea30632bf0934acc675b28ae2107203db.json`
a trouvé 186 demandes de preuve, dont 176 textes distincts : 9 876 octets de texte
contre 9 570 après déduplication. **Le gain de 306 octets n’explique pas et ne
résout pas la surcharge.** Ce comptage est un diagnostic de transport, pas une
nouvelle validation ni un changement d’ancrage des preuves.

## Options comparées

| Option | Fidélité | Coût / reprise | Décision |
|---|---|---|---|
| Dédupliquer les textes des questions | Possible sans perte, avec identités séparées | Gain observé minime ; questions toujours à résoudre individuellement | Ne pas traiter comme solution principale |
| Transport textuel avec références à des blocs identiques | Reconstruction octet pour octet obligatoire, sans résumé | Peut réduire JSON échappé et sources déjà présentes dans les patches ; gain à mesurer | Prochain prototype pur et isolé |
| Décisions séparées par critère et synthèse courante | Couverture et interactions inter-critères doivent rester explicites | Plus d’appels ; ne résout pas automatiquement sept appels restants | Option architecturale si transport insuffisant |
| Retirer les traces ou garder seulement leurs résumés | Perd potentiellement la preuve nécessaire | Gain apparent, résultat non démontré | Refusé |

## Prochaine vérification avant implémentation

1. Mesurer séparément contenu canonique, enveloppe JSON, preuves présentes à
   plusieurs endroits et partie maximale de la décision finale.
2. Réutiliser les primitives de transport sans perte existantes lorsque leur
   contrat convient. Tester la reconstruction exacte sur UTF-8, chemins et
   caractères échappés, hunks, suppression et données incomplètes.
3. Comparer le coût des appels réellement réservés sur un Store jetable ; un
   nombre théorique d’octets ne démontre pas un gain d’appels.
4. Versionner toute nouvelle représentation : ne jamais changer la reconstruction
   d’un prompt déjà ancré dans un ancien journal.
5. Vérifier séparément capacité de la réponse finale et couverture de toutes les
   réserves : réduire le message d’entrée ne suffit pas à garantir une réponse
   structurée admissible.

Aucun changement du plafond de 71, aucun nouvel appel réel ni installation de
protocole supplémentaire n’est autorisé par ce document seul. L’implémentation
locale du correctif reste dans le périmètre APEX déjà demandé.

## Prototype hors moteur — 27 septembre, 13:43 UTC

`tools/review-transport/probe.py` implémente le transport expérimental des chaînes
longues avec blocs UTF-8 encadrés par taille et empreinte, chemins de remplacement
explicites et dictionnaire des duplications exactes. La reconstruction vérifie
chaque bloc puis l’empreinte canonique du document complet. Six tests passent :
UTF-8/échappement, duplications, faux marqueur dans le contenu, troncature,
corruption, référence manquante/dupliquée et entrées scalaires. Les petits contenus
peuvent coûter davantage ; ce cas est testé et n’est pas masqué.

Sur le contexte historique du refus `review-db7a817c9acff024a689360c` :
1 827 067 octets JSON canoniques deviennent 1 743 630 octets de transport,
soit 83 437 octets économisés avec reconstruction exacte. Ce contexte historique
n’est pas le sous-ensemble frais du candidat actuel : cette mesure ne prouve donc
pas que les sept appels disponibles suffisent. Elle ne comprend pas les consignes
IA et le schéma. Aucun fournisseur lancé, aucune preuve historique modifiée.

Conclusion limitée : la suppression de l’échappement et des répétitions exactes
est réalisable et testée, mais son gain seul ne démontre pas le déblocage E6.
Avant toute intégration, mesurer le transport des paquets réellement restants et
la décision finale, notamment les sources communes aux patches. Aucun changement
du moteur installé n’est livré par ce prototype.

## Mesure sur les pièces réellement restantes — 13:51 UTC

Une nouvelle entrée de diagnostic en lecture seule, `planning review-dossier`,
exporte le contexte canonique de la tentative bloquée sans autoriser de revue.
L’API de planification expose la même opération avec `action=review-dossier`.
Le test CLI/HTTP couvre contenu canonique identique, budget nul, tâche absente,
absence de dépense et conservation du travail/de la tentative. Le fichier exporté
contient des sources et rapports privés : le conserver localement.

Sur E6 actuel, export par le binaire de développement et comparaison avant/après :
révision457 et état identiques, candidat01a7a591e86d088f190861bfe86b170d71fc98ea.
La reconstruction diagnostique retrouve les cinq groupes historiques0,1,3,4,5,
275pièces totales et exactement les1 101 354octets frais annoncés par le moteur.
Les149pièces fraîches occupent1 187 620octets JSON ; le prototype produit
1 148 247octets avec reconstruction exacte, soit39 373octets de gain.

**Cette piste seule ne suffit pas :** même sans consignes ni schémas,1 148 247octets
ne tiennent pas dans cinq inspections de196 608octets. Le minimum théorique est
six inspections plus deux appels finaux, donc huit appels pour sept disponibles.
Ce n’est pas une estimation opérationnelle à huit appels : le vrai découpage peut
en nécessiter davantage. Aucun protocole expérimental envoyé à un fournisseur.

Les prochaines options à examiner sont la représentation sans perte des sources
communes aux patches et la capacité d’entrée réellement documentée du fournisseur.
La constante192Kio est une limite actuelle du moteur ; ne pas l’assimiler sans
preuve à la fenêtre de contexte du modèle, et ne pas la relever arbitrairement.
Une décision décomposée reste une autre architecture, avec son coût et ses
obligations de couverture explicites.

## Capacités : distinguer octets, tokens et configuration du client

La [fiche officielle GPT-5.6 Sol](https://developers.openai.com/api/docs/models/gpt-5.6-sol),
consultée le27septembre2026, annonce une fenêtre API de1 050 000tokens et une sortie
maximale de128 000tokens. Elle indique aussi un palier de prix au-delà de272 000tokens
d’entrée. Cela ne démontre pas la configuration effective du client Codex utilisé
par Swarm : son cache local du même jour annonce272 000tokens et95% de fenêtre
effective pour `gpt-5.6-sol`, sans plafond de sortie explicite dans cet objet.

La constante Swarm de192Kio porte sur les **octets du message**, pas sur les tokens.
Elle ne peut donc pas être justifiée simplement en la présentant comme la limite
du modèle. La piste suivante est un calcul d’admission adapté au fournisseur,
avec comptage de tokens documenté, réserves de sortie/raisonnement et provenance
figée dans le plan. Une nouvelle version doit préserver les anciens prompts et
leurs empreintes. Ne pas changer la fenêtre du client, le budget ou un palier de
coût pour faire passer le dossier sans conditions vérifiées. Aucun de ces réglages
n’a été modifié pendant cette analyse.

## Validation de l’export en lecture seule

`planning review-dossier` et son équivalent HTTP exportent les preuves courantes
sans appel fournisseur ni mutation, y compris lorsque le budget est épuisé.
Test ciblé CLI/HTTP : réussi. Suite Go complète : réussie en 289,091 s.
Le premier passage avait échoué sur une comparaison des espaces JSON dans le test ;
la comparaison corrigée conserve tous les champs et compare leur JSON canonique.
`go vet`, contrôle de contrat et `git diff --check` : réussis.
Ce diagnostic est disponible dans les sources ; le serveur installé reste en
révision `9da8a65`. E6 reste bloquée : cet export ne constitue pas une validation.

## Comptage local de tokens — 27 septembre

Le tokenizer publié `tiktoken==0.14.0` associe `gpt-5.6-sol` à `o200k_base`.
Le diagnostic `tools/review-transport/token_probe.py` utilise cette association
sans repli implicite, conserve exactement le texte UTF-8 et ne contacte aucun
modèle. La version 0.12.0 initialement essayée ne connaît pas ce nom ; elle a
refusé, plutôt que choisir un tokenizer arbitraire.

| Contenu mesuré | Octets UTF-8 | Tokens du texte |
| --- | ---: | ---: |
| Pièces fraîches, JSON agrégé | 1 187 620 | 352 892 |
| Pièces fraîches, prototype sans perte | 1 148 247 | 327 361 |
| Contexte canonique intégral | 1 841 967 | 532 669 |

Ces mesures ne contiennent ni les consignes et schémas de chaque paquet, ni
l’enveloppe du client. Le contexte intégral dépasse à lui seul la fenêtre locale
observée de 272 000 tokens. Les pièces fraîches exigent toujours un découpage ;
en revanche, le nombre de tokens n’impose pas à lui seul huit inspections.
Cela justifie d’examiner une admission par tokens, pas de relever silencieusement
la constante de 192 Kio ni d’affirmer que sept appels suffisent déjà.

### Correction à développer

1. Figer dans une nouvelle version du plan la capacité effective, le tokenizer,
   ses paramètres et les réserves ; conserver les anciens prompts identiques.
2. Compter le **message complet et son schéma**, garder également un plafond
   mémoire en octets et le plafond de réponse par pièce (`managedFragmentReplyFits`).
3. Vérifier les paquets et les deux appels finaux, avec les réserves historiques
   et preuves originales sélectionnables ; refuser avant réservation si un message
   ne tient pas. Un petit nombre d’appels n’est pas une preuve de complétude.
4. Recetter sur Store isolé : mauvaise identité/capacité, tokenizer absent,
   corruption, réouverture, anciens journaux inchangés, budgets conservés et vrai
   défaut rejeté par le fournisseur simulé. Ne lancer aucune revue réelle pour
   développer cette étape.

Les 12 tests des outils diagnostiques passent (1,006 s). Aucun code d’admission
moteur ni réglage fournisseur modifié par ce lot ; aucune nouvelle acceptation.

## Primitive moteur d’admission — étape en cours

Implémentation dans `managed_review_input_budget.go` : contrat de capacité
versionné, comptage séparé du message complet et du schéma avec vocabulaire embarqué,
plafond mémoire distinct, UTF-8 strict et réserves explicites. Aucun calcul ne
réserve un appel, ne contacte un fournisseur ou ne modifie le Store.

Choix : [tokenizer Go embarqué](https://github.com/tiktoken-go/tokenizer), épinglé
à v0.6.2, plutôt qu’un sous-processus Python et un vocabulaire à télécharger sur
la machine de production. v0.8.1 exige Go 1.26 ; v0.6.2 conserve notre contrat
Go 1.24. Les deux dépendances ajoutées sont le tokenizer et regexp2. Aucun
changement de version Go dans le dépôt.

Le contrat expérimental est volontairement restreint à `gpt-5.6-sol`,
`o200k_base/tiktoken-go-v0.6.2`, une fenêtre au plus de 272 000 tokens et 95 %
effectifs, au moins 32 768 tokens réservés au client et 65 536 à la sortie et au
raisonnement, et au plus 1 Mio de texte UTF-8. Ces réserves sont une politique
conservatrice, pas une mesure des instructions cachées du fournisseur. Toute
capacité inconnue, réserves insuffisantes, dépassement mémoire/tokens ou UTF-8
invalide produit un refus. Les séquences sans séparation de plus de 8 Kio sont
refusées avant le BPE pour borner son travail quadratique, sans texte amputé.

**Pas encore branché sur les revues actives.** Il reste à lier cette capacité
à la configuration effective du client, l’ancrer dans un nouveau plan de
fragments, puis appliquer le même contrat aux inspections et aux deux appels
finaux. Les versions historiques gardent leur limite et leurs empreintes.
Ne pas annoncer E6 débloquée à partir de cette primitive seule.

Les tests ciblés comparent le Go à six vecteurs produits indépendamment par
`tiktoken==0.14.0`, y compris Unicode, chaînes ressemblant à des tokens spéciaux,
diff et JSON. Ils vérifient qu’un message de 360 000 octets peut tenir en tokens,
qu’un schéma trop grand bloque, que les deux plafonds sont distincts, que les
contrats invalides sont rejetés et que plusieurs compteurs restent indépendants.
Tests ciblés : PASS 1,049 s. Suite Go complète : PASS 292,158 s. Race ciblée :
PASS 14,443 s ; initialisation concurrente seule : PASS 1,244 s. `go vet`,
contrat agent et `git diff --check` : PASS. Pas de test fournisseur réel.
