# Auto-revue finale IA3a

17 octobre 2026, Codex dans le contexte producteur, **non indépendant**.
Base : `baseline.json` et préimages, pas le seul HEAD Git du checkout partagé.

Verdict : **composant interne vérifié sur les contrôles ciblés**, sous les
préconditions décrites. Pas d'acceptation globale ou de lanceur public activé.

Les cinq sources préexistantes ont été comparées à leurs préimages ; les quatre
nouveaux fichiers, leurs producteurs et consommateurs ont été relus. Les sources
mémoire et lanceurs sont inchangés. Les trois fichiers de revue d'un essai rejeté
sont restaurés exactement : l'interdiction de répéter un protocole inchangé
n'a été ni supprimée ni contournée.

La publication vérifie le plan sous l'écrivain SQLite avant intention. Les
inspections copient une transaction cohérente. L'évaluation d'instance garde
sa réservation SQL pendant runtime, avec callback fermé et pure évaluation ;
`withCaller`, les fonctions de contrôle/rejeu et les méthodes runtime utilisées
ne réentrent pas dans Store. Cette trace locale ne certifie pas tous les
callbacks futurs. Les erreurs de transaction ne retournent pas de décision
positive. La nouvelle course est conservée avant/après, sur cage réelle.

La projection conserve les champs inconnus par défaut, trie les objets sans
float64 et retire seulement les champs d'exécution identifiés. Les triggers
maintiennent la génération dans la transaction Work, sans octroi implicite.
Migration, rollback, dépassement entier, restitution après réouverture, altération,
hiérarchie et conversion sont couverts ; les sélections demeurent des données.

232 noms ciblés PASS, dont 32 du plan ; 84 noms de ce sous-ensemble rejoués sous
`-race`, sans FAIL/SKIP. Statique/compilation réussies, 1 056 entrées figées.
Les deux nouveaux schémas sont inspectés ; six rendus précédents sont réutilisés
à sources et empreintes identiques. Le coût de la sérialisation n'est pas mesuré.

Réserve explicite : un ancien test de reprise de revue panique aussi sur les
sources initiales. Son attente de reprise par simple consigne contredit le
contrat protégé actuel. Le diagnostic, l'essai rejeté et la condition de reprise
sont conservés ; la sélection finale ne représente pas une suite globale verte.
Ce point demeure ouvert au chantier moteur/reprise, propriétaire Codex.

Autres limites : émetteurs humains et témoins de fixtures, absence de témoin
indépendant du watermark du plan, aucun contrôle de restauration physique
globale déployé, compte hôte non protégé, appartenance/contenu exact de création
et engagement d'effet encore absents. F01–F07, IDA, admission publique, droits
mémoire et RETEX global restent ouverts. La documentation ne prouve pas leur
réception par un autre chat ou une régulation autonome.
