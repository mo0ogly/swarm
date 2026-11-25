# APEX — avis IA sur l’architecture d’un workflow

| ID | Critère observable |
| --- | --- |
| W1 | L’aide FR/EN mène à un brouillon qui compare séquence, hiérarchie, combinaison et preuves insuffisantes. |
| W2 | Le brouillon de réévaluation conserve le texte saisi et demande les nouvelles preuves, l’avis précédent et le motif d’un maintien ou d’un changement. |
| W3 | L’envoi explicite utilise la conversation existante, les droits, la provenance et le budget du fournisseur. Les boutons de brouillon n’émettent aucun appel et l’avis laisse l’adoption à l’opérateur. |
| W4 | Le parcours navigateur expédie les deux types de brouillons via le serveur protégé, vérifie l’aide et les langues, puis la livraison qualifie le binaire effectivement servi. |

Le parcours conversationnel existant conserve les réponses et expose leur contexte. La solution réutilise son envoi explicite et compose un message modifiable. Un moteur de score aurait besoin d’entrées structurées et de comparateurs établis pour arbitrer les cas, et une bascule automatique modifierait les responsabilités d’un plan approuvé. Cette livraison n’introduit ni score, ni surveillance en arrière-plan, ni mutation de plan par l’avis.

La grille part des dépendances et de leur stabilité, des preuves aux remises, des arbitrages de périmètre, des reprises, de la validation et du coût ou de la latence mesurés. Un changement de faits peut provoquer un maintien, une révision motivée ou un avis indéterminé. Une preuve manquante ne justifie pas une bascule. Les cas de référence sont une chaîne linéaire à étapes stables, des branches spécialisées qui nécessitent un arbitrage, et un changement documenté de l’autonomie ou des dépendances qui justifie un nouvel avis. [Guide Anthropic, p. 15 et 18–19](https://resources.anthropic.com/hubfs/Building%20Effective%20AI%20Agents-%20Architecture%20Patterns%20and%20Implementation%20Frameworks.pdf).

Recette publique existante : `preparation-calls-rust`. Le fournisseur détenu vérifie l’envoi, la persistance, le rejeu et les contrôles de la conversation. Un avis rendu par ce double ne mesure pas la qualité de recommandation d’un modèle réel.

## Avis automatique dans Conduite

| ID | Critère observable |
| --- | --- |
| A1 | Le graphe enregistré produit un avis motivé, sans appel IA ni écriture à la lecture. |
| A2 | Une nouvelle révision relue produit un nouvel avis et présente le caractère historique d’une validation précédente. |
| A3 | La validation explicite exige les droits de mutation de mission, le jeton de session, la révision et l’empreinte exactes. Un rejeu de la même validation conserve le reçu. |
| A4 | Une soumission périmée ou indéterminée est refusée. Les tâches, la configuration de la mission et ses autorités restent celles du plan enregistré. |
| A5 | L’interface FR/EN et le diagramme montrent le parcours, puis le binaire qualifié reproduit la lecture et la validation sur une mission de recette isolée. |

Le moteur ne possède pas de bascule générique entre les deux modes. L’objet
calculé est un avis. Garder l’audit IA manuel permet d’examiner des contraintes
qualitatives, mais un déclenchement automatique du fournisseur exigerait un
contrat d’autorisation et de budget. Le calcul structurel est retenu pour la
relecture automatique. Une chaîne et une hiérarchie peuvent coexister à des
niveaux différents. Le critère hiérarchique réclame une preuve de coordination,
en plus de la présence de branches et de périmètres. Les arcs de prérequis
d’exigences à producteur unique comptent dans le graphe. Une provenance
ambigüe produit un avis indéterminé. Aucun seuil de coût,
nouveau délai ni surveillance en arrière-plan.

Ordre de vérification : classification et contre-exemples, contrôle de l’écriture
dans la transaction, parcours HTTP et navigateur avec révision modifiée, puis
qualification du binaire livré sur la racine locale. Une validation d’avis
n’autorise jamais une conversion ou un lancement de mission.
