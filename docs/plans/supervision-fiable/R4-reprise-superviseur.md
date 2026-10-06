# R4 — reprise du superviseur

Le producteur R4 a terminé sa première tentative avec tests ciblés et rapport. Les contrôles ont été renouvelés en liant les sept fichiers applicatifs/test réellement changés et la recette. Un avis a été rejeté pour citation reformattée (majuscule initiale au lieu de la minuscule du texte source) ; une déclaration non mise en forme de la limite NOT TESTED a été ajoutée au rapport. Aucun nouveau producteur et aucun plafond augmenté.

Le superviseur a détecté une régression de l’aide générale : --lang en --help affichait encore le texte français parce que la clé du catalogue était ancienne. Correction attribuée au superviseur : synchronisation de la clé/traduction dans locales/en.json, régénération web/i18n-en.js, nouveau supervision_r4_i18n_test.go. Le premier test a échoué sur son attente erronée du libellé show|decide ; le libellé réel est show <agent> | decide <agent>. Test corrigé code 0 et node tests/i18n_test.cjs code 0. Le contrôle R4 lie ces entrées supplémentaires et exécute le nouveau test avant nouvelle revue.

Les fichiers communs ont invalidé les preuves antérieures de R1 et R3. Leurs contrôles et revues ont été renouvelés publiquement, sans nouvelle production. Les gates delivery puis acceptations de R1, R3 et R4 ont été enregistrées avec preuves fraîches. La comparaison externe de fournisseurs demeure NOT TESTED ; coûts inconnus conservés. L’ancien travail reste 5/6 non clôturé.

Le serveur est lancé avec /tmp/swarm-supervision-r4-final-candidate sur 18792. T5 a démarré automatiquement et le responsable a été repris publiquement ; aucune pause humaine n’est requise. La suite finale reste à exécuter par le contrôle moteur préautorisé. Les fichiers de T5 actif n’ont pas été modifiés par le superviseur.

Limite complémentaire observée : les nouvelles données JSON de relais gardent leur forme canonique. Le refus sans quota actif lu avec --lang en affichait encore un motif source français (hors aide générale corrigée) ; ne pas déduire une couverture exhaustive des nouveaux messages de la recette de versions. À intégrer à une vérification bilingue ciblée si la clôture prétend couvrir tous les messages du nouveau relais.
