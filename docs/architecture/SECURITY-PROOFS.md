# Sécurité et preuves

[Plan et reste à faire](../plans/actions-confiance/SECURITY-PROOFS-APEX.md).

Le premier raccord fournit une observation en lecture seule : configuration BLP
courante et inventaire déclaratif embarqué. La page `/security-proofs.html` est
accessible depuis le cockpit Rust et Actions de confiance. Elle requiert la session
native puis un jeton Trust I3/admin courant ; une session Authority seule ne crée
aucune identité Trust. Le jeton reste en mémoire, effacé au changement de page ou
par le bouton dédié ; tout changement de saisie invalide les résultats affichés.

CLI : `swarm-rust trust security-show --root ROOT --input FILE --json`, avec
`SWARM_TRUST_TOKEN` dans l'environnement et `FILE` contenant `{"schema_version":1}`.
API : POST `/api/v1/trust-actions?action=security-show`, même corps, avec session,
CSRF et authentification Bearer. Les champs inconnus, résultats ou chemins fournis
sont refusés. Les verrous partagés, validation d'état, liaison Root et erreurs de
fermeture restent ceux du protocole Trust. Aucune publication ni mutation.

Le rapport `security-observation-v1` expose un domaine `blp`, la capacité BLP
existante, les métadonnées de compilation, les empreintes SHA256 de l'inventaire
et du catalogue embarqués, les noms/propriétaires/lots ouverts des frontières et
les références de recettes. Les références sont des données, jamais des commandes.
L'empreinte d'observation est le digest canonique du rapport avant ajout de ce champ.
Elle n'est ni une signature, ni une empreinte du binaire, ni une preuve de profil.
Le rapport n'inclut ni habilitations, attributions, raisons, chemins privés,
contenus projet ou jetons. Il demeure réservé à l'administration existante.

Configuration, vérification et qualification sont indépendantes :
`runtime_enforced:false`, `verification.result:NOT_TESTED`, fraîcheur `unknown`,
`evidence_loaded:false`, qualification `not_qualified`, revue indépendante et
certification externe `false`. Ces états signifient qu'aucune preuve n'est chargée
par cette observation, pas que les anciennes recettes n'ont jamais été exécutées.
Une politique absente, courante ou à renouveler est distinguée ; les 25 frontières
OPEN sont celles de l'inventaire compilé, pas une mesure dynamique exhaustive.

L'administration de politique garde son formulaire existant (lecture/preview/apply)
dans Actions de confiance. Lancement de recettes, import et fraîcheur de preuves,
droits de consultation distincts, export et décision de qualification restent les
lots L04–06 du plan. Biba et composition restent à raccorder. Aucun bouton ne permet
de déclarer une certification ou d'activer la protection globale.

Recettes maintenues : `security-proofs-engine.json` pour la projection ; le parcours
public doit aussi vérifier CLI/HTTP/UX et les octets de l'image effectivement livrée.
Une vérification de composant n'est ni installation ni acceptation indépendante.


## Livraison du premier raccord

La version dev.336 est activée localement. Les recettes de lecture moteur et
CLI/HTTP/UX sont vérifiées sur des fixtures détenues avec la même image normale ;
77 assertions navigateur et 22 CLI/HTTP, plus 56 sur le dialogue Version existant.
La recette réelle Authority reste ouverte sur connexion humaine ; aucun jeton
technique n’a été promu en session humaine. Voir le plan pour les preuves et limites.
Mainteneur : relire le catalogue de recettes et actualiser les entrées/compteurs
du manifeste d’inventaire dans le même incrément. Utiliser le navigateur installé
explicitement et le collecteur commun, sans supposer le cache Puppeteer.
