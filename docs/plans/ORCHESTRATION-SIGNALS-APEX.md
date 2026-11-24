# APEX · Remontées de statut dans Conduite

| ID | Critère observable |
| --- | --- |
| R1 | Une remontée de clôture ou d’arbitrage conserve la source, le destinataire, la date et le signal enregistré. Les retours des workers sont identifiables séparément. |
| R2 | Une réception indique le contexte de planification du destinataire auquel le signal est lié. Un traitement indique la décision enregistrée qui cite ce signal. Un signal en attente n’invente ni réception ni décision. |
| R3 | La vue Équipe et validation ouvre les remontées dans une fenêtre de lecture, avec libellés FR/EN, navigation clavier et refus explicites. |
| R4 | Une recette isolée suit un refus de sous-orchestrateur, sa remise à root, la décision de root et la reprise du sous-orchestrateur, puis rapproche la lecture HTTP et la modale du même signal. |

La lecture directe de la dernière livraison d’un périmètre perdrait les
réceptions historiques. Une table de suivi supplémentaire dupliquerait
l’état et introduirait des divergences de transaction. La projection retenue
rapproche la boîte du travail, les contextes historiques liés aux claims et
les décisions canoniques, dans une même lecture du Root. L’ordre logique
des révisions prévaut sur les dates. Le contenu des prompts reste privé.

Une transmission signifie un dépôt enregistré dans la boîte du destinataire.
Une réception signifie l’inclusion vérifiée dans son contexte enregistré.
Le traitement signifie que le destinataire a cité le signal dans une décision
appliquée. Une décision sans opération peut traiter le signal sans résoudre
l’incident. Les anciens signaux peuvent manquer de preuves. Aucune lecture
n’acquitte, ne relance ou ne modifie une mission.

Ordre d’exécution : reproduire une remontée avec le fournisseur inerte,
raccorder la lecture protégée sous Journaux, vérifier les états intermédiaires,
tester l’affichage, puis installer le binaire qualifié. Biba sert de témoin
de lecture seule, avec ses retours non traités et son conducteur arrêté.
