# PAEA — incrément40 : paquet opérateur protégé

39livré157 :51cas composés/7CLI/56UI. Lecture effaçable et précontrôle d'écriture
disponibles. Propriétaire Codex direct, aucun GLM/délégation/commit/push.

Choix : publier directement plusieurs fichiers de service avant intention, ou
conserver d'abord la matière38 exacte dans Documents39, avec pin connu avant IO.
Deuxième route. Cette tranche couvre uniquement le paquet opérateur atomique et
son chargement sous pin ; publication des journaux/fichiers de service41 ensuite.
Seal consomme Prepared, conserve les octets privés effaçables et expose un pin de
données à protéger indépendamment. PackageStore possède Documents déjà ouvert et
lié à ses descripteurs. create singleton NOREPLACE refuse tout existant, même les
mêmes octets ; load exige le pin extérieur, jamais appris du fichier. Aucun reset,
rotation, KDF implicite, grant/application de journal ni auto-adoption.

Critères : création/relecture/réouverture exactes et refus duplicat ; pin incorrect/
altération/refus de parsing sans retour de clé ; ENOSPC/partial/fsync/close avant
rename et reprise avec le même Seal ; Visible après directorysync, Durable après
unlockEIO restent connus en erreur. Chaque réservation finit explicitement, deux
causes conservées. Observation de lecture distingue tentative inconnue/octets lus/
pin validé/décodage, y compris après échec d'unlock ; Prepared rejeté est effacé.
Une lecture réussie ne réacquitte pas implicitement une durabilité de publication
et ne prouve aucune fraîcheur/autorité humaine. Pas de nouvel endpoint natif ici.

Avant41 : séparation physique du vault opérateur et des racines service à vérifier
par les objets effectivement ouverts ; clé métier jamais copiée dans le service.
La présente API ne gère aucun layout de service ni compte OS.41doit persister pin/
intention indépendants avant effets, tracer fichiers/journaux/UNKNOWN et reprise
exacte, puis CLI/UX/réglages. PA02partiel,0/17/E2/E3/PA13 et rotation/confinement ouverts.


## État vérifié40, livraison ouverte

12tests nommés/2étapes PASS, sources stables/intégrité VERIFIED ; Rustdoc privé -Dwarnings et revue locale des octets PASS. Les21cas lecture/écriture39 sont réutilisables : sources config/transitives inchangées, nouveau consommateur vérifié séparément. Diagrammes FR/EN rendus/relus. Deux builds normaux refusés : premier compilation0 mais quatre entrées effectives du chantier réglages changées pendant le build ; deuxième Cargo échoue sur RootInstanceIdentity non exporté dans deux nouveaux consommateurs de préparation. Originaux copiés dans les preuves40. Un garde avant relance a refusé une précondition encore changée sans lancer Cargo. Native40 a été arrêté par assertion avant tout scénario ; aucun UI40/installation40 affirmé. Le dernier serveur qualifié reste157/39. Propriétaire de livraison Codex, dépendance du chantier préparation concurrent : export réel/consommateurs cohérents puis profil normal stable. Poursuivre les travaux indépendants ; suite41 doit acquitter explicitement la durabilité du paquet avant les effets sur les fichiers de service et vérifier leur séparation physique.
