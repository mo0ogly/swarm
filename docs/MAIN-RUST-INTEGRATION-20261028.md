# Intégration Rust dans main

La fusion conserve les deux historiques. Le moteur Go supprimé par la migration n’est pas réintroduit. Les contrôles SQLite sont ceux du moteur Rust, persistés et accessibles par les réglages natifs ; les guides et recettes Go sont historiques. Les ajouts de formation/Docker de main sont conservés. Les préimages historiques ne sont pas fusionnées textuellement : la variante migration reste à son chemin, la variante main reste dans son commit parent.

Le vérificateur de méthodes appelle le test Rust agent_workflow, exige les neuf contrats nommés et refuse une collecte vide. Les profils CI sans debug/incrémental correspondent à la construction locale qualifiée, sans changer les politiques métier ni retirer de contrôle. Les changements non committés du poste sont préservés. CI checks/install restent nécessaires avant fusion ; aucune acceptation indépendante ou activation d’identité réelle ne découle de ce document.
