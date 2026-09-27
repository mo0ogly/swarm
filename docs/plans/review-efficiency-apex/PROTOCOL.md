# Revue : refus explicites et coût observable / Review protocol

## Français

Les nouveaux plans complets utilisent le protocole 2 ; les plans réutilisant des observations historiques emploient le protocole 3 décrit dans DIFFERENTIAL.md. Un refus exige une pièce identifiée, une ligne, une citation exacte, une explication, une condition de reproduction et le comportement attendu. La ligne désigne le texte original pour une source, ou la ligne du patch pour un diff. Le moteur vérifie l'attribution et la structure ; cela ne prouve pas automatiquement que le diagnostic du modèle est juste.

Les raisons synthétiques sont conservées pour le transport final. Les défauts détaillés sont fournis dans `defects` dès le premier appel, conservés dans le journal ancré et exposés dans le motif de refus. Jusqu'à quatre défauts par réponse, avec 8 Kio supplémentaires réservés ; les autres incertitudes restent `unknown`. Les journaux de protocole 1 restent lisibles, sans transformation en preuves nouvelles.

Deux interruptions de la même inspection avec les mêmes preuves et le même délai empêchent une nouvelle reprise identique. Les dépenses restent enregistrées. Une modification explicite du délai suit la voie de reprise existante ; le remplacement de fournisseur conserve ses contrôles séparés. Cette règle ne prétend pas reconnaître toutes les boucles sémantiques.

Prévol en lecture seule, disponible même avec budget insuffisant :

```sh
swarm --root /chemin/projet --json planning review-cost WORK --input request.json
```

`request.json` : `{"task_id":"TASK"}`.

HTTP : `GET /api/v1/planning?work=WORK&task=TASK&action=review-cost` via la session authentifiée habituelle.

Le résultat expose le candidat, le protocole, les pièces/octets, fichiers du diff, fichiers modifiés depuis le refus, inspections/appels finaux et budget disponible. Cette opération ne modifie aucun quota et ne lance aucun fournisseur. RD3 conserve certains groupes d’observations historiques lorsque toutes leurs entrées sont identiques, avec provenance intacte et nouvel examen obligatoire des impacts ; ce n’est pas une fermeture statique complète des dépendances. Les champs fits_budget et transport_ready distinguent plafond d’appels et capacité des messages. Une estimation de 15 appels n'est pas une garantie de conclusion.

## English

New full inspection plans use protocol 2; plans retaining historical observations use protocol 3, described in DIFFERENTIAL.md. A refusal requires an artifact, original line, exact quote, causal explanation, reproduction condition and expected behavior. Source line numbers refer to decoded source contents; diff line numbers refer to the supplied patch. Structural validation does not prove that the model's diagnosis is correct.

Detailed defects arrive in the first response, are retained in the anchored journal and appear in the refusal reason. The protocol allows four detailed defects with an additional 8 KiB response allowance. Additional uncertainties remain unknown. Version 1 journals remain readable as historical records.

Two interruptions of the same inspection with unchanged evidence and timeout block another identical retry. Spent calls are never refunded. Explicit timeout changes and provider replacement retain their separate recovery checks. This does not detect every semantic loop.

The read-only `planning review-cost` command and authenticated HTTP `action=review-cost` endpoint expose review size and required/available calls even when the budget is insufficient. They neither authorize spending nor start providers. Protocol 3 can retain local observations with unchanged complete input groups and original provenance. A fresh impact review is mandatory; this does not certify complete dependency closure. Check both fits_budget and transport_ready before retrying. The final independent verdict must still cover the current candidate and its controls.

## Protocole 4 — admission en tokens

Un nouveau plan peut utiliser le protocole 4 lorsque la capacité locale du client
Codex configuré est établie et que le plan historique ne tient pas. Le plan ancre
le modèle, le tokenizer, la capacité, les réserves et l’identité du client. Tous
les messages complets et schémas sont comptés avant réservation ; les plafonds
mémoire, réponse et appels restent applicables. Les observations historiques et
leurs réserves restent sous leurs identités originales. Les anciens plans v1–v3
ne sont ni réécrits ni implicitement convertis.

### English

Protocol 4 admits new review messages using an anchored token capacity from the
configured Codex client's bundled model catalog. Complete prompts and schemas
are counted before spending; memory, response and call limits remain enforced.
Unknown capabilities fail closed. Historical observations retain their original
identities and unresolved obligations. Existing v1–v3 plans are not rewritten.
A successful size preflight does not mean the independent review will pass.
