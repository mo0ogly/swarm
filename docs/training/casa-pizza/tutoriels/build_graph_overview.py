"""Assemble the French graph introduction from genuine, refreshed UI captures."""
import subprocess
import tempfile
from pathlib import Path

from PIL import Image
from build_media import ROOT, card, module, step, timestamp


def main():
    # Fixed screenshot sequences, not a continuous recording. Total: 63 seconds.
    scenes = [
        ('overview-01', 3, 'Des tâches reliées par des prérequis.'),
        ('overview-02', 4, 'Zoomer pour lire une étape.'),
        ('overview-03', 7, 'Replier les branches. Garder le contexte.'),
        ('overview-04', 4, 'Passer du graphe au détail d’une tâche.'),
        ('overview-04', 5, 'Commande et suivi : deux prérequis, deux tâches en aval.'),
        ('15-humaine-fr', 10, 'Revue humaine ou contrôles structurés.'),
        ('14-tache-fr', 4, 'Écrire le critère de réussite avant le contrôle.'),
        ('19-commande-fr', 4, 'Déclarer les fichiers examinés.'),
        ('19-commande-fr', 3, 'python3 : un programme autorisé et visible.'),
        ('19-commande-fr', 3, 'Les arguments sont exacts. Un par ligne.'),
        ('16-controle-fr', 10, 'Un délai configurable : 60 secondes.'),
        ('17-apercu-fr', 6, 'Examiner l’effet du changement avant de confirmer.'),
    ]
    mod = module('graph-overview', 'Le graphe et ses contrôles', '', [], '', '', '', '')
    results = [
        'Les flèches vont du prérequis vers la tâche qui en dépend.',
        'Ajustez le zoom pour lire les informations de chaque tâche.',
        'Réduisez une branche pour vous concentrer sur une partie du travail.',
        'Le détail rassemble les critères, les dépendances et les actions.',
        'Les prérequis déterminent les étapes à terminer en premier.',
        'Choisissez les contrôles adaptés aux critères de la tâche.',
        'Le critère précise le résultat attendu.',
        'Le contrôle examine les fichiers indiqués.',
        'Choisissez le programme qui réalise le contrôle.',
        'Saisissez chaque argument sur sa propre ligne.',
        'Adaptez le délai à la durée prévue du contrôle.',
        'Vérifiez la portée et les preuves à renouveler.',
    ]
    mod['steps'] = [step(frame, text, '', result, '')
                    for (frame, _, text), result in zip(scenes, results)]
    cues = ['WEBVTT', '']
    elapsed = 0
    with tempfile.TemporaryDirectory(prefix='overview-') as directory:
        directory = Path(directory)
        cards = []
        for i, (_, seconds, text) in enumerate(scenes):
            target = directory / f'{i:02d}.png'
            card(mod, mod['steps'][i], 'fr', i).save(target)
            cards.append((target, seconds))
            cues += [f'{timestamp(elapsed)} --> {timestamp(elapsed + seconds)}', text, '']
            elapsed += seconds
        listing = directory / 'concat.txt'
        listing.write_text(''.join(f"file '{p}'\nduration {seconds}\n" for p, seconds in cards)
                           + f"file '{cards[-1][0]}'\n")
        subprocess.run(['ffmpeg', '-nostdin', '-hide_banner', '-loglevel', 'error', '-y',
                        '-f', 'concat', '-safe', '0', '-i', str(listing), '-t', str(elapsed),
                        '-vf', 'fps=12', '-c:v', 'libx264', '-preset', 'fast', '-crf', '26',
                        '-pix_fmt', 'yuv420p', '-movflags', '+faststart',
                        str(ROOT / 'media/graph-overview-fr.mp4')], check=True)
        Image.open(cards[0][0]).save(ROOT / 'media/graph-overview-fr-poster.png')
    (ROOT / 'media/graph-overview-fr.vtt').write_text('\n'.join(cues))
    print(f'Graph introduction: {elapsed}s, 1280×900, real refreshed screenshots.')


if __name__ == '__main__':
    main()
