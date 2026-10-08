# IA sur site : configuration Docker locale

Swarm appelle l’IA depuis son serveur. L’accès réseau, les certificats et les
proxies doivent donc fonctionner dans le conteneur, même si `curl` fonctionne
sur le poste hôte. Le navigateur ne réalise pas cet appel à l’IA.

## Ajouter un override

`compose.override.yaml` complète `compose.yaml` sans modifier la configuration
publiée. Créez-le à la racine du dépôt, à côté de `compose.yaml`. Ce fichier,
les certificats sous `certs/` et `deploy/install.env` sont ignorés par Git.
Conservez les réglages locaux déjà présents ; ne remplacez pas tout le fichier.

Les valeurs de `environment` remplacent celles du service de base. Les volumes
se combinent selon leur destination. Certaines listes se combinent aussi : ne
répétez pas `security_opt: [no-new-privileges:true]`, déjà présent dans le fichier
principal, sous peine d’une erreur de valeurs dupliquées.

L’installateur utilise explicitement `compose.yaml`. Pour appliquer l’override,
utilisez explicitement les deux fichiers dans les commandes suivantes :

```sh
docker compose --env-file deploy/install.env \
  -f compose.yaml -f compose.override.yaml config --quiet

docker compose --env-file deploy/install.env \
  -f compose.yaml -f compose.override.yaml \
  up -d --force-recreate swarm
```

`restart` ne recharge pas les variables ni les montages. Recréer le conteneur
applique les changements ; ajouter `--build` si le code ou l’image a changé.
Arrêtez les missions actives avant de remplacer le service. Gardez les mêmes
`SWARM_PROJECT` et `SWARM_AGENT_HOME` pour conserver les données persistantes.
`swarm.sh` gère le serveur natif ; il ne redémarre pas Docker.

## Autorité interne déjà approuvée par l’hôte

Sur un hôte Linux disposant de `/etc/ssl/certs/ca-certificates.crt`, vous pouvez
monter ce bundle de confiance en lecture seule :

```yaml
services:
  swarm:
    environment:
      SSL_CERT_FILE: /etc/ssl/certs/ca-certificates.crt
    volumes:
      - type: bind
        source: /etc/ssl/certs/ca-certificates.crt
        target: /etc/ssl/certs/ca-certificates.crt
        read_only: true
        bind:
          create_host_path: false
```

Le chemin `source` appartient à l’hôte ; `target` et `SSL_CERT_FILE` appartiennent
au conteneur. Vérifiez que le bundle existe et contient l’autorité approuvée.
Il conserve les autorités publiques déjà présentes sur l’hôte.

## Bundle spécifique au déploiement

Si vous ne voulez pas monter le bundle de l’hôte, obtenez auprès de votre équipe
infra un bundle PEM validé, avec les autorités nécessaires, et placez-le dans
`certs/onprem-ca-bundle.pem`. Ne copiez pas simplement le certificat du serveur.

```yaml
services:
  swarm:
    environment:
      SSL_CERT_FILE: /etc/swarm-certs/ca-bundle.pem
    volumes:
      - type: bind
        source: ./certs/onprem-ca-bundle.pem
        target: /etc/swarm-certs/ca-bundle.pem
        read_only: true
        bind:
          create_host_path: false
```

Le fichier doit être lisible par l’utilisateur du conteneur. Incluez les autres
autorités nécessaires dans ce bundle ; ne désactivez pas la vérification TLS.

## DNS, proxy ou serveur sur le poste hôte

Pour un DNS interne manquant, un mapping temporaire peut être ajouté au même
service. Remplacez le nom et l’adresse d’exemple par ceux validés par votre infra :

```yaml
services:
  swarm:
    extra_hosts:
      - "ai.example.internal:192.0.2.10"
```

Cela ne change pas le nom HTTPS : le certificat doit toujours correspondre au
nom de serveur utilisé. L’adresse d’exemple n’est pas une adresse de service.

Pour un proxy, ajoutez `HTTPS_PROXY`, `HTTP_PROXY` et `NO_PROXY` sous
`environment`, selon les règles du réseau. Un service interne accessible
directement doit figurer dans `NO_PROXY`. Conservez les identifiants de proxy
hors Git et ne partagez pas une sortie complète de `docker compose config` :
elle peut révéler des valeurs sensibles.

Le Compose livré utilise le réseau hôte Linux : `127.0.0.1` dans le conteneur
peut joindre un serveur IA écoutant sur le loopback de cet hôte. Cette règle ne
s’applique pas à une configuration modifiée utilisant un réseau bridge.

## Vérifier et connecter l’IA

Après validation et recréation, testez depuis le conteneur :

```sh
docker compose --env-file deploy/install.env \
  -f compose.yaml -f compose.override.yaml \
  exec swarm curl --connect-timeout 5 --max-time 15 \
  -sS -o /dev/null -w 'HTTP %{http_code}\n' \
  https://ai.example.internal/v1/models
```

Remplacez l’URL par celle documentée par votre service. `401` sans clé indique
que TLS et HTTP ont abouti mais que l’authentification est requise. Ce n’est pas
un test complet du modèle. Un `404` peut indiquer un chemin incompatible.

Dans **IA et connexions**, choisissez une API compatible avec adresse
personnalisée, sa base (souvent `/v1`), l’identifiant exact du modèle et sa clé.
Swarm ajoute `/chat/completions` à cette base ; ne saisissez pas déjà ce suffixe.
Testez puis **enregistrez la connexion** : le test seul n’enregistre pas la clé.
La connexion est conservée dans le projet persistant ; utilisez **Copier le
diagnostic** pour transmettre les étapes DNS, TCP, TLS et HTTP.

Pour vérifier le projet monté, utilisez :

```sh
docker compose --env-file deploy/install.env \
  -f compose.yaml -f compose.override.yaml \
  exec swarm sh -c 'printf "Projet interne : %s\n" "${SWARM_ROOT:-/workspace}"'
```

`SWARM_PROJECT` dans `deploy/install.env` désigne le projet sur l’hôte,
`SWARM_ROOT` le projet dans le conteneur. Ils ne sont pas forcément le dépôt.
Les méthodes utilisées en préparation doivent exister dans ce projet et leurs
liens symboliques doivent rester dans son périmètre. Pour APEX, vérifiez
`.claude/skills/apex/SKILL.md` et `tools/agent-workflows/CONTRACT.md` sous cette
racine. Copiez les méthodes requises depuis le dépôt si le projet ne les possède
pas, en préservant les fichiers personnalisés.

Avant de partager des logs, retirez clés, liens de session privés et informations
internes. Les fichiers spécifiques à une organisation restent locaux.
