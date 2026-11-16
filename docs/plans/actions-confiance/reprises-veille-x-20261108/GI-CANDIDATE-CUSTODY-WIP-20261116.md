# WIP — transition du candidat du producteur Gi.eth

Source revue, non compilée et non vérifiée en exécution : ne pas fusionner comme validée.

Le contre-exemple R09-CUSTODY-1 de la première sauvegarde7088121 est résolu à la source par deux détenteurs distincts. sources::request acquiert/rafraîchit le candidat préclaim hors SQL ; son callback garde les transactions de préparation/revendication et renseigne Work/claim connus avant sa fermeture. sources::job acquiert ensuite le détenteur de la review réellement enregistrée/running hors SQL ; son callback garde owner/fraîcheur/horloge/expiration/historique. Aucune égalité, protection I0, politique de budget/TTL ou limite supprimée. Les erreurs après claim ne signifient pas rollback.

```mermaid
flowchart LR
 A[Préclaim sans review] --> B[Revendication enregistrée]
 B --> C[Fermeture préclaim]
 C --> D[Holder de la review exacte]
 D --> E[Contrôles et expiration]
 E --> F[Fermeture et rapports]
```

Revue09 APPROVE_SOURCE_ONLY : SHA6b3d83798f27900c0c035197cf96ef41e870693c99adeef52a15b554e0fc9263. Source5c0e1ab29b4eb87673b126f2314c34d5c8ef343d5ca957894d8a8b567f1ff14a ; preuves privées /run/media/fpizzi/SWARM/swarm-evidence/root00-gi-custody-transition-u41ymcya. Une ligne blanche séparatrice supprimée, aucun autre corps de fonction changé.

Restent requis : qualification du nouveau SDK, vrai témoin managed valide avant mauvais/nonrunning review et cleanup conservant knownWork, puis reprise contrôlée. Aucun build/test/W/SDK/F/installation/acceptation réalisé pour ce candidat. Le Work refusé w-83e055eae3e47c27a0f5043c, dernière lecture rev14/calls0, n’est pas modifié. Les anciens scénarios ne sont pas rejoués.
