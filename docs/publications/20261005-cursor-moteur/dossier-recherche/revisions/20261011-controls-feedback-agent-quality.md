# Contrôles, rétroactions et qualité du travail des agents

Question de recherche : les contrôles, le replan et la transmission des
apprentissages améliorent-ils les résultats des agents, et à quel coût ?

[Documentation d'architecture avec six exemples et Mermaid](../../../../architecture/CONTROLS-AND-FEEDBACK.md)
— [English version](../../../../en/CONTROLS-AND-FEEDBACK.md).

Hypothèse : des contrôles attribués et une rétroaction exploitable augmentent la
fiabilité lorsque l'incident reçu conduit à une décision, une correction et un
effet vérifié. La prévention des récidives exige ensuite un apprentissage reçu
et utile aux tâches suivantes. Un journal ou des instructions modifiées ne
suffisent pas à établir cet effet.

Les exemples séparent observations natives, corrections qualifiées avec doubles
et illustrations à éprouver : preuve T2 mal attribuée, inventaire incomplet,
blocage SQL d'une correction, quota Claude, développement parallèle et processus
métier. Aucun résultat global favorable n'est déduit d'un test local.

Protocole proposé : comparer des tâches comparables avant/après, avec moteur,
modèles, entrées, budgets et interventions attribués. Mesurer acceptation au
premier passage, récidives par cause, relances humaines, délai et coût par résultat
accepté, chaînes réellement résolues et coût des contrôles. Conserver refus,
attentes et reprises dans les dénominateurs. Les améliorations de vitesse et
coût restent des hypothèses tant que cette comparaison n'est pas réalisée.

[Historique et preuves des corrections](20261011-instruction-learning-retex.md).
Le RETEX doit confronter l'hypothèse aux effets observés et permettre à
l'utilisateur de choisir les évolutions suivantes.
