# Reprise des campagnes de facturation — 10 octobre 2026

## Objectif et candidat

Après installation du moteur main `3faa8a9f0b6c7e606b3a7bda2c847a96060a9779`, vérifier que le banc historique fonctionne encore puis rejouer les **31 combinaisons F4e historiquement exclues** : 26 erreurs SQLite et cinq délais de progression. La campagne historique de 9 000 cases et la vérification de 1 700 cases sont conservées intégralement. Le rejeu est un nouvel échantillon ciblé, pas une correction rétroactive des observations.

Deux précontrôles réels sur le binaire installé ont passé : règlement nominal exact et indisponibilité API sans second producteur (`tests.test_run_s`, 2 tests, 23,443 s).

## Méthode figée avant le rejeu

- Graines, clés et faute issues du JSON d'analyse des 32 exclusions, sans sélection selon les nouveaux résultats.
- Exécutions séquentielles, une par combinaison, racine isolée par exécution ; délai et budgets existants du banc inchangés.
- Empreintes du binaire, du banc, du diagnostic historique et du lanceur enregistrées ; reprise refusée si elles changent.
- Aucun accès direct à la base du moteur. Les données du grand livre synthétique restent gérées par le banc.
- Les racines sont conservées ; les IDs des missions sont recueillis par CLI publique après arrêt ordonné.
- Au premier échec ou nettoyage défectueux : arrêt et diagnostic ; aucun rejeu identique automatique.
- Cas F7 clé tentative/graine 1010 **non testé dans ce lot** : le banc utilise pour sa preuve une lecture directe de `.swarm/state.db`. Une preuve publique doit être préparée avant ce rejeu.

## Critères et interprétation

Publier tous les statuts, paiements inexacts, doublons, impayés, faux succès, preuve d'ordre de mutation, fraîcheur de `prepare`, arrêt attribué et événements de dépendance. Sous F4e un arrêt qui empêche le paiement est un résultat attendu ; « OK » signifie exécution mesurable, pas factures toutes réglées.

Le responsable, le préparateur et le relecteur restent **scriptés**. Aucun appel LLM n'est engagé par ce rejeu ; il ne valide ni un collectif autonome ni une transaction bancaire réelle. Une disparition des délais ne permet pas d'attribuer l'effet au seul correctif SQLite : plusieurs corrections ont changé depuis la campagne historique. Un passage isolé n'est pas une estimation de fiabilité.

## Suite de l'étude

1. Publier le bilan ciblé et les cas non couverts sans modifier les journaux historiques.
2. Préparer la preuve publique du cas F7 et le diagnostic QR3 des cinq pertes de progression.
3. Actualiser la conception S-collectif sur l'API actuelle ; vérifier les adaptateurs et leur isolement avant tout appel réel.
4. Pilote de deux exécutions sans faute avec responsable/préparateur/relecteur réels, consommation par rôle et critères d'arrêt explicites ; la campagne de 90 essais attend les mesures de ce pilote.
