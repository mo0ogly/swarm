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
