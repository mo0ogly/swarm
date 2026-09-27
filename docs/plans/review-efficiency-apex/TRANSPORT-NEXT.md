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
