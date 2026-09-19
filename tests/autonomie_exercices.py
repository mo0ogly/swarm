#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Exercices d'agents sur le systeme autonome du compagnon Swarm.

Chaque exercice met le moteur dans une situation reelle, avec de vrais
processus d'agents (fournisseurs de test locaux, aucun modele appele), puis
mesure ce que le moteur a fait seul et ce qu'il a laisse a l'humain.

Ce qui est verifie n'est pas qu'il ne plante pas : c'est qu'il tient ses
promesses, y compris celles qui l'empechent d'agir.
"""
import argparse, json, os, shutil, subprocess, sys, tempfile, time, uuid
from pathlib import Path

class Exercice:
    def __init__(self, binaire):
        self.binaire = os.path.abspath(binaire)
        self.racine = tempfile.mkdtemp(prefix='swarm-exo-')
        self.cli('init')

    def cli(self, *args, entree=None):
        cmd = [self.binaire, '--root', self.racine, '--json', *args]
        if entree is not None:
            cmd += ['--input', '-']
        r = subprocess.run(cmd, input=json.dumps(entree) if entree is not None else None,
                           capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError(f"{' '.join(args)} -> {r.returncode} : {r.stderr.strip()[:300]}")
        s = r.stdout.strip()
        return json.loads(s) if s.startswith(('{', '[')) else None

    def muter(self, args, revision, champs):
        d = {'schema_version': 1, 'event_id': uuid.uuid4().hex, 'expected_revision': revision}
        d.update(champs)
        return self.cli(*args, entree=d)

    def revision(self, w):
        return self.cli('work', 'show', w)['work']['revision']

    def travail(self, w):
        return self.cli('work', 'show', w)['work']

    def fournisseur(self, nom, corps):
        p = Path(self.racine) / f'{nom}.py'
        p.write_text(corps)
        p.chmod(0o700)
        conf = Path(self.racine) / '.swarm/providers.json'
        cfg = json.loads(conf.read_text()) if conf.exists() else {'schema_version': 1, 'providers': {}}
        cfg['providers'][nom] = {'command': '/usr/bin/python3', 'args': [str(p)], 'env_allow': []}
        conf.write_text(json.dumps(cfg))

    def demarrer_web(self):
        """Le cockpit sert la seule vue qui agrege decisions et couts : la lire
        par HTTP exerce le transport reel plutot qu'un raccourci interne."""
        self.serveur = subprocess.Popen([self.binaire, '--root', self.racine, 'web'],
                                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        ligne = self.serveur.stdout.readline()
        import re
        m = re.search(r'http://\S+', ligne)
        if not m:
            raise RuntimeError('cockpit non demarre : ' + ligne)
        self.session = m.group(0)
        self.base = self.session.split('/session/')[0]
        import urllib.request, http.cookiejar
        self.jar = http.cookiejar.CookieJar()
        self.http = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(self.jar))
        self.http.open(self.session).read()

    def vue(self, chemin):
        with self.http.open(self.base + chemin) as r:
            return json.loads(r.read().decode())

    def arreter_web(self):
        if getattr(self, 'serveur', None):
            self.serveur.terminate()
            self.serveur.wait(timeout=10)

    def profil(self, w, tache, fournisseur, role='worker'):
        return self.cli('profile', w, tache, entree={
            'provider': fournisseur, 'role': role, 'workspace': self.racine})

    def attendre(self, predicat, limite=45, quoi='condition'):
        fin = time.time() + limite
        while time.time() < fin:
            if predicat():
                return True
            time.sleep(0.4)
        raise AssertionError(f'delai depasse : {quoi}')

    def nettoyer(self):
        shutil.rmtree(self.racine, ignore_errors=True)


AGENT_QUI_LIVRE = '''import sys,json,time,pathlib
sys.stdin.read()
print(json.dumps({"type":"assistant","message":{"content":[{"type":"tool_use","id":"t","name":"Bash","input":{"description":"Produire le rapport","command":"true"}}]}}),flush=True)
print(json.dumps({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t","content":"ok"}]}}),flush=True)
pathlib.Path(%r).write_text("# Rapport\\nTravail effectue par la fixture.\\n")
print(json.dumps({"type":"result","usage":{"input_tokens":10,"output_tokens":5},"total_cost_usd":0.25}),flush=True)
'''

AGENT_MUET = '''import sys,json
sys.stdin.read()
print(json.dumps({"type":"result","usage":{"input_tokens":1,"output_tokens":1}}),flush=True)
'''

AGENT_EN_ECHEC = '''import sys,json
sys.stdin.read()
print(json.dumps({"type":"result","is_error":True}),flush=True)
sys.exit(3)
'''


def exercice_relais(x, res):
    """Le conducteur relaie un rapport prouve, et rien de plus."""
    w = x.muter(['work', 'create'], 0, {'title': 'Relais', 'objective': 'o', 'scope': 's',
                                        'criteria': ['c']})['work']
    x.muter(['task', 'add', w['id']], w['revision'],
            {'id': 'r1', 'title': 'Produire', 'deliverable': 'rapport',
             'criteria': ['c'], 'owner': 'session', 'next': 'lancer'})
    (Path(x.racine) / 'docs').mkdir(exist_ok=True)
    x.fournisseur('livre', AGENT_QUI_LIVRE % str(Path(x.racine) / 'docs/r1.md'))
    x.muter(['agent', 'start', w['id']], x.revision(w['id']),
            {'task_id': 'r1', 'provider': 'livre', 'workspace': x.racine,
             'role': 'worker', 'timeout_seconds': 60, 'capture_output': True})

    x.attendre(lambda: x.travail(w['id'])['tasks'][0]['status'] in ('submitted', 'blocked'),
               quoi='fin de la tentative')
    t = x.travail(w['id'])['tasks'][0]
    res.mesure('relais automatique du rapport prouve', t['status'] == 'submitted',
               f"statut obtenu : {t['status']}")
    res.mesure('soumis n est jamais accepte', t['status'] != 'accepted',
               'le conducteur ne doit jamais accepter')
    res.mesure('aucune gate posee par le moteur', t.get('gate') is None,
               'une gate automatique serait une validation sans humain')


def exercice_refus_relais(x, res):
    """Sans rapport lisible, le conducteur refuse et motive."""
    w = x.muter(['work', 'create'], 0, {'title': 'Refus', 'objective': 'o', 'scope': 's',
                                        'criteria': ['c']})['work']
    x.muter(['task', 'add', w['id']], w['revision'],
            {'id': 'm1', 'title': 'Rien produire', 'deliverable': 'rapport',
             'criteria': ['c'], 'owner': 'session', 'next': 'lancer'})
    x.fournisseur('muet', AGENT_MUET)
    x.muter(['agent', 'start', w['id']], x.revision(w['id']),
            {'task_id': 'm1', 'provider': 'muet', 'workspace': x.racine,
             'role': 'worker', 'timeout_seconds': 60, 'capture_output': True})
    x.attendre(lambda: x.travail(w['id'])['tasks'][0]['status'] in ('submitted', 'blocked'),
               quoi='fin de la tentative muette')
    t = x.travail(w['id'])['tasks'][0]
    res.mesure('sans rapport, la tache reste bloquee', t['status'] == 'blocked',
               f"statut obtenu : {t['status']}")
    res.mesure('le refus porte son motif', bool((t.get('blocker') or '').strip()),
               'un blocage sans motif oblige a fouiller les journaux')


def exercice_suspension(x, res):
    """La suspension prime sur le niveau autonome."""
    w = x.muter(['work', 'create'], 0, {'title': 'Suspension', 'objective': 'o', 'scope': 's',
                                        'criteria': ['c']})['work']
    x.muter(['task', 'add', w['id']], w['revision'],
            {'id': 's1', 'title': 'Partir', 'deliverable': 'rapport',
             'criteria': ['c'], 'owner': 'session', 'next': 'lancer'})
    x.fournisseur('muet', AGENT_MUET)
    x.profil(w['id'], 's1', 'muet')
    x.cli('control', w['id'], entree={'command': 'pause'})
    x.cli('autonomy', w['id'], 'autonome', '2')
    x.cli('dispatch', w['id'])
    t = x.travail(w['id'])['tasks'][0]
    res.mesure('aucun depart pendant la suspension', t['status'] == 'todo',
               f"statut obtenu : {t['status']}")
    x.cli('control', w['id'], entree={'command': 'unpause'})
    x.attendre(lambda: x.travail(w['id'])['tasks'][0]['status'] != 'todo',
               quoi='depart apres reprise')
    res.mesure('la reprise relance l ordonnancement',
               x.travail(w['id'])['tasks'][0]['status'] != 'todo',
               'sans cela, l absence de depart ne prouverait rien')


def exercice_echecs_repetes(x, res):
    """Deux echecs automatiques arretent les departs et escaladent."""
    w = x.muter(['work', 'create'], 0, {'title': 'Echecs', 'objective': 'o', 'scope': 's',
                                        'criteria': ['c']})['work']
    x.muter(['task', 'add', w['id']], w['revision'],
            {'id': 'e1', 'title': 'Echouer', 'deliverable': 'rapport',
             'criteria': ['c'], 'owner': 'session', 'next': 'lancer'})
    x.fournisseur('casse', AGENT_EN_ECHEC)
    x.profil(w['id'], 'e1', 'casse')
    x.cli('autonomy', w['id'], 'autonome', '2')

    tentatives = 0
    fin = time.time() + 60
    while time.time() < fin:
        x.cli('dispatch', w['id'])
        time.sleep(1.2)
        t = x.travail(w['id'])['tasks'][0]
        n = len(t.get('attempts') or [])
        if n == tentatives and n >= 2:
            break
        tentatives = n
    t = x.travail(w['id'])['tasks'][0]
    n = len(t.get('attempts') or [])
    res.mesure('les departs automatiques s arretent apres deux echecs', n <= 2,
               f'{n} tentatives lancees ; la borne evite la boucle a la charge de l humain')
    decisions = [d for d in x.vue(f"/api/v1/snapshot?work={w['id']}")['decisions'] if not d.get('resolved_at')]
    res.mesure('les echecs repetes remontent a l humain', bool(decisions),
               'sans escalade, la tache s arreterait en silence')


def exercice_un_sujet_une_carte(x, res):
    """Un meme sujet ne produit qu une carte, quelle que soit la repetition."""
    w = x.muter(['work', 'create'], 0, {'title': 'Cartes', 'objective': 'o', 'scope': 's',
                                        'criteria': ['c']})['work']
    x.muter(['task', 'add', w['id']], w['revision'],
            {'id': 'c1', 'title': 'Echouer', 'deliverable': 'rapport',
             'criteria': ['c'], 'owner': 'session', 'next': 'lancer'})
    x.fournisseur('casse', AGENT_EN_ECHEC)
    x.profil(w['id'], 'c1', 'casse')
    x.cli('autonomy', w['id'], 'autonome', '2')
    for _ in range(4):
        x.cli('dispatch', w['id'])
        time.sleep(1.0)
    ouvertes = [d for d in x.vue(f"/api/v1/snapshot?work={w['id']}")['decisions'] if not d.get('resolved_at')]
    par_tache = {}
    for d in ouvertes:
        par_tache.setdefault(d.get('task_id'), []).append(d)
    pire = max((len(v) for v in par_tache.values()), default=0)
    res.mesure('un sujet ne donne qu une carte', pire <= 1,
               f'{len(ouvertes)} cartes pour {len(par_tache)} sujet(s) ; au pire {pire} sur un meme sujet')


def exercice_cout_non_invente(x, res):
    """Un cout non rapporte s ecrit comme tel, jamais zero."""
    w = x.muter(['work', 'create'], 0, {'title': 'Cout', 'objective': 'o', 'scope': 's',
                                        'criteria': ['c']})['work']
    x.muter(['task', 'add', w['id']], w['revision'],
            {'id': 'k1', 'title': 'Sans cout', 'deliverable': 'rapport',
             'criteria': ['c'], 'owner': 'session', 'next': 'lancer'})
    x.fournisseur('muet', AGENT_MUET)
    x.muter(['agent', 'start', w['id']], x.revision(w['id']),
            {'task_id': 'k1', 'provider': 'muet', 'workspace': x.racine,
             'role': 'worker', 'timeout_seconds': 60, 'capture_output': True})
    x.attendre(lambda: x.travail(w['id'])['tasks'][0]['status'] != 'running',
               quoi='fin de la tentative sans cout')
    vue = x.vue(f"/api/v1/snapshot?work={w['id']}")
    texte = vue.get('cost_text', '')
    res.mesure('un cout non rapporte ne devient pas 0,00',
               '0.00' not in texte and '0,00' not in texte,
               f'texte du cout : {texte!r}')
    res.mesure('les tentatives muettes sont comptees a part',
               'sans coût rapporté' in texte or 'non rapporté' in texte,
               f'texte du cout : {texte!r}')


class Resultats:
    def __init__(self):
        self.lignes = []

    def mesure(self, quoi, tenu, detail=''):
        self.lignes.append({'controle': quoi, 'tenu': bool(tenu), 'detail': detail})
        marque = 'TENU  ' if tenu else 'ROMPU '
        print(f'  {marque} {quoi}' + (f' — {detail}' if detail else ''))

    def rompus(self):
        return [l for l in self.lignes if not l['tenu']]


EXERCICES = [
    ('relais du rapport prouve', exercice_relais),
    ('refus motive sans rapport', exercice_refus_relais),
    ('la suspension prime', exercice_suspension),
    ('bornage des echecs repetes', exercice_echecs_repetes),
    ('un sujet, une carte', exercice_un_sujet_une_carte),
    ('le cout inconnu ne devient pas zero', exercice_cout_non_invente),
]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--binary', required=True)
    ap.add_argument('--report', required=True)
    a = ap.parse_args()
    res = Resultats()
    echecs = []
    for nom, fn in EXERCICES:
        print(f'\n== {nom} ==')
        x = Exercice(a.binary)
        try:
            x.demarrer_web()
            fn(x, res)
        except Exception as e:
            print(f'  ERREUR  {nom} : {e}')
            echecs.append({'exercice': nom, 'erreur': str(e)})
        finally:
            x.arreter_web()
            x.nettoyer()
    rompus = res.rompus()
    rapport = {'status': 'PASS' if not rompus and not echecs else 'FAIL',
               'controles': res.lignes, 'erreurs': echecs}
    Path(a.report).write_text(json.dumps(rapport, ensure_ascii=False, indent=1))
    print(f"\n=== {len(res.lignes)} controles | {len(rompus)} rompu(s) | {len(echecs)} erreur(s) ===")
    for l in rompus:
        print('  ROMPU : ' + l['controle'] + ' — ' + l['detail'])
    return 0 if rapport['status'] == 'PASS' else 1


if __name__ == '__main__':
    sys.exit(main())
