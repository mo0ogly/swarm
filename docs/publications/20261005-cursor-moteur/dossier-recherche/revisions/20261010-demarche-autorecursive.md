# Démarche autorécursive obligatoire et correction de nos remontées

L’utilisateur a dû pousser plusieurs diagnostics et corrections du moteur.
Il demande une démarche autorécursive documentée, obligatoire dans les conversations
Codex et Claude, pour ne pas laisser une erreur au stade du signalement.
Sauf demande explicite d’arrêt ou de report, poursuivre les actions autorisées,
vérifier puis réexaminer le besoin initial. Une dépendance externe reste attribuée
et visible ; continuer le travail indépendant disponible.

Le [contrat d’architecture](../../../../architecture/AUTORECURSIVE-RECOVERY.md)
et sa [version anglaise](../../../../en/AUTORECURSIVE-RECOVERY.md) décrivent boucle,
rôles, progression, escalades et vérifications, avec trois Mermaid chacun.
Les consignes communes, APEX, replan et le checkpoint de travail sont actualisés.
La préférence personnelle est enregistrée dans les instructions globales Codex
et importée par Claude Code ; une skill personnelle commune porte la méthode.
Ces réglages locaux ne constituent pas une preuve de reprise autonome dans Swarm.

## Réexaminer la correction précédente

La précédente transmission de consignes conservait les contrats, preuves et
budgets, mais ajoutait le diagnostic à la fin de longs champs `next`. L’inspection
de `planningContextLimit` et `recoveryTaskFocus` révèle des projections de tête
limitées respectivement à 1 000 et 600 caractères. La présence de consignes dans
la réponse publique `work show` ne prouvait donc pas leur visibilité dans ces
contextes. Ce défaut appartient à notre méthode de transmission et à son contrôle.

Correction bornée : placer un résumé opératoire en tête de `next`, puis conserver
les détails et les consignes précédentes. Vérifier la visibilité dans les deux
projections, avec acteur, événement et révision. Les empreintes et événements du
relevé JSON adjacent conservent l’historique ; aucun plafond n’est augmenté.
Cette vérification du contenu ne prouve pas encore son exploitation par l’IA.

## Application au blocage T2

La revue indépendante protège le candidat `a27e175eb4f3e38f9cede1c888956a5ca5a0310f`.
Les traces de reproduction du juge sont disponibles dans la
[note de traçabilité](20261010-tracabilite-developpement.md) : six tests de premier
niveau et 13 PASS avec sous-cas, origine opérateur. La suite complète interrompue
n’est pas déclarée réussie. Le diagnostic transmis doit distinguer exécution
locale, reçu moteur et avis indépendant, puis sélectionner la reprise publique
correspondant au verdict final. Ne pas répéter un contrôle inchangé ni fabriquer
une acceptation à partir d’une assertion du producteur.

La demande et les corrections opérateur sont conservées comme interventions
humaines. Elles fournissent un scénario T3–T7 : erreur connue, diagnostic ou
transmission défectueux, nouvelle preuve discriminante, correction attribuée et
réexamen de l’objectif. Mesurer effets, inconnus, décisions des superviseurs,
coût et interventions ; ne pas déduire l’autonomie de la rédaction du protocole.
