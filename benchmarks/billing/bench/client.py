"""Client HTTP minimal (bibliothèque standard) et faute d'environnement."""
import json
import urllib.error
import urllib.request

from bench import config


class Unavailable(Exception):
    """Service de paiement injoignable ou en erreur 5xx : faute d'environnement, pas de contenu."""


def get(base, path):
    with urllib.request.urlopen(base + path, timeout=config.HTTP_TIMEOUT_S) as response:
        return json.loads(response.read())


def post(base, path, body, token=None):
    headers = {"Content-Type": "application/json"}
    if token is not None:
        headers["X-Settlement-Token"] = token
    request = urllib.request.Request(base + path, data=json.dumps(body).encode(), headers=headers, method="POST")
    with urllib.request.urlopen(request, timeout=config.HTTP_TIMEOUT_S) as response:
        return json.loads(response.read())


def snapshot(base):
    try:
        return get(base, "/snapshot")
    except urllib.error.HTTPError as e:   # avant URLError : HTTPError en hérite
        if e.code >= 500:
            raise Unavailable(f"service de paiement indisponible ({e.code})") from e
        raise
    except (ConnectionError, TimeoutError, urllib.error.URLError) as e:
        raise Unavailable(f"service de paiement injoignable ({e})") from e
