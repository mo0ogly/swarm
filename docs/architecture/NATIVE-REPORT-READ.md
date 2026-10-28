# Lecture native des rapports

État : réglages candidats, lecteur/API/CLI et livraison non terminés.

Le groupe de réglages47 introduit deux plafonds distincts, dans la configuration
moteur commune et son catalogue Admin : report_preview_max_bytes limite les
octets retenus pour affichage ; report_verification_max_bytes limite les octets
lus pour vérifier intégralement le rapport privé attribué. Le défaut initial
262144 d’affichage figure uniquement dans engine-settings.json ; le défaut de
vérification est null, explicitement sans plafond. Aucun fallback dans le moteur.

Une absence historique ou un groupe partiel refuse la consommation. Null retire
uniquement le plafond concerné. Zéro ne retient aucun octet de texte, ou refuse
la vérification d’un rapport non vide. La validation des chaînes décimales, des
valeurs négatives et de la représentation machine appartient au moteur commun.
API, CLI et dialogue Admin doivent conserver la même validation, les révisions,
le preview sans mutation et la relecture après application. Ce groupe indépendant
n’adopte pas implicitement les politiques d’exécution absentes d’un ancien profil.

La lecture prévue autorise les documents publics contenus dans docs, ou le seul
rapport privé attribué à la tâche et à sa tentative courante. La vérification SHA
complète ne peut réussir sur un extrait. Le plafond du contexte de planification
reste une autre politique. Les erreurs d’accès, de source et de fermeture ne
doivent devenir ni texte vide ni succès partiel. Aucun lancement d’agent, budget,
acceptation ou droit de restauration ne suit la lecture. Ces derniers critères
sont à implémenter et vérifier ; les tests de réglages ne les prouvent pas.

~~~mermaid
flowchart LR
    Defaults[Versioned initial configuration] --> Settings[Persisted shared settings]
    Admin[Admin modal / API / CLI] --> Preview[Validate and preview]
    Preview --> Apply[Fresh revision and checked application]
    Apply --> Settings
    Settings --> Caps[Explicit preview and verification caps]
    Caps -. reader pending .-> File[Attributed regular file]
    File -. complete SHA when required .-> View[Display text is not proof acceptance]
~~~
