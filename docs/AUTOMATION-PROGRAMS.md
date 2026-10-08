## Administration web et CLI C04

Dans le cockpit, passez en mode expert puis ouvrez **Programmes**. Saisissez un
nom, une mission existante, le type, le fuseau IANA et la date locale. Cliquez
**Prévisualiser sans effet**, examinez les instants UTC, le profil, les limites,
l’autorisation et le coût (inconnu si le fournisseur ne le rapporte pas), puis
**Créer désactivé**. La création ne démarre pas un agent. **Activer**, **Suspendre**
et **Archiver** portent une révision ; un conflit exige de recharger. Une archive
est définitive. **Annuler cette attente** s’applique seulement avant prise en
charge ou effet. Une occurrence traitée n’est pas une mission validée.

L’aide textuelle s’ouvre dans une modale, se ferme avec Échap et restitue le focus.
Les états, actions, motifs d’autorisation et erreurs sont traduits dans la langue
du cockpit ; les codes machine restent inchangés dans les réponses JSON pour les
scripts. En anglais, par exemple, une erreur de fuseau indique explicitement
`explicit IANA timezone required; UTC is accepted`.
La section **Réglages opérationnels versionnés** expose les huit paramètres de
bail, échéance, occurrences et événements externes. Enregistrer ajoute une
révision et son auteur ; les écritures concurrentes sur la même révision sont
refusées sans écraser ni les valeurs enregistrées par l’autre opérateur ni la
saisie visible. Rechargez alors la page, relisez la nouvelle révision et
réappliquez volontairement votre modification. Cela ne relève aucun budget et
ne modifie aucune tentative active.

```sh
swarm automation help
swarm --lang en automation help
swarm automation list
swarm automation show PROGRAMME
swarm automation preview --input programme.json
swarm automation create --input creation.json
swarm automation enable PROGRAMME --input revision.json
swarm automation pause PROGRAMME --input revision.json
swarm automation archive PROGRAMME --input revision.json
swarm automation cancel OCCURRENCE --input revision.json
swarm automation params show
swarm automation params apply --input reglages.json
```

`creation.json` contient `schedule` (le document prévisualisé) et `preview_token`
retourné par l’aperçu. `revision.json` contient `expected_revision` de l’objet.
Les réglages contiennent `schema_version: 1`, `expected_revision` et `values`
avec les huit champs courants. Les adaptateurs CLI/HTTP utilisent le même service ;
le HTTP local conserve session et CSRF, sans connecteur ni port public nouveau.
L’hôte doit toujours appeler le tick autorisé : cette page n’installe pas un poller.

## Captures et revue indépendante

Les quatre captures FR/EN sombre/État sont fournies comme fichiers PNG déclarés sous `docs/screenshots/` dans les entrées de contrôle. Le moteur vérifie leurs empreintes, dimensions et tailles avant la revue. Un simple lien dans un rapport ne joint aucune image. Codex reçoit les octets via `--image` dans un répertoire privé temporaire, effacé après le processus ; Claude les reçoit dans le message structuré. La revue garde ses outils désactivés et le même budget. Une adaptation API ou fournisseur non vérifiée est refusée. Les fixtures ne prouvent pas la disponibilité d’un abonnement.
