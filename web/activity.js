'use strict';
// Fil d'activité : la ligne de vie du travail. Chaque entrée dit qui a agi —
// le moteur ou vous — parce que c'est la seule chose que l'autonomie rend
// difficile à savoir après coup.

// Référence de visite figée à l'ouverture du travail. Si on la relisait à
// chaque rafraîchissement, le repère glisserait sous les yeux du lecteur — une
// visite écrite ailleurs (cockpit terminal, second onglet) le ferait sauter en
// tête et lui ferait perdre ce qu'il était en train de parcourir.
let filVisiteReference = null;

let filEntrees = [];
let filCurseur = '';
let filSuite = false;
let filTravail = '';
let filEnCours = false;

function filMarqueur(origine) {
  // Le symbole double le mot, il ne le remplace pas : une couleur seule ne se
  // lit pas de la même façon par tout le monde.
  const e = node('span', origine === 'moteur' ? '▸ moteur' : '● vous', 'fil-origine');
  e.dataset.origine = origine;
  return e;
}

function filHeure(iso) {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
}

function filJour(iso) {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? '' : d.toLocaleDateString('fr-FR');
}

function filLigne(entree, jourPrecedent) {
  const li = node('li', undefined, 'fil-entree');
  const jour = filJour(entree.at);
  if (jour && jour !== jourPrecedent) li.append(node('p', jour, 'fil-jour'));
  const tete = node('div', undefined, 'fil-tete');
  const heure = node('time', filHeure(entree.at), 'fil-heure');
  heure.dateTime = entree.at;
  tete.append(heure, filMarqueur(entree.origin), node('span', entree.label, 'fil-label'));
  li.append(tete, node('p', entree.message || 'Motif non renseigné', 'fil-message'));
  return { li, jour };
}

// Âge lisible : « il y a 2 h » situe mieux qu'une date absolue au moment de
// reprendre un travail.
function filDepuis(iso) {
  const minutes = Math.round((Date.now() - Date.parse(iso)) / 60000);
  if (!Number.isFinite(minutes) || minutes < 1) return "à l'instant";
  if (minutes < 60) return 'il y a ' + minutes + ' min';
  const heures = Math.round(minutes / 60);
  if (heures < 24) return 'il y a ' + heures + ' h';
  return 'il y a ' + Math.round(heures / 24) + ' j';
}

function filSeparateurVisite() {
  const li = node('li', undefined, 'fil-visite');
  li.append(node('span', 'votre dernière visite · ' + filDepuis(filVisiteReference)));
  return li;
}

function filRendre() {
  const liste = $('fil-entrees');
  liste.replaceChildren();
  let jour = '';
  let separateurPose = false;
  for (const entree of filEntrees) {
    // Le trait se pose devant la première entrée antérieure à la visite de
    // référence : tout ce qui est au-dessus est arrivé depuis.
    if (!separateurPose && filVisiteReference && entree.at < filVisiteReference) {
      liste.append(filSeparateurVisite());
      separateurPose = true;
    }
    const { li, jour: j } = filLigne(entree, jour);
    jour = j;
    liste.append(li);
  }
  $('fil-plus').hidden = !filSuite;
  const etat = $('fil-etat');
  if (filEntrees.length) {
    etat.textContent = filSuite ? '' : 'Début du fil.';
  } else {
    etat.textContent = 'Aucune activité enregistrée pour ce travail. Les départs, relais et décisions apparaîtront ici.';
  }
}

async function filCharger({ suite = false } = {}) {
  if (!work || filEnCours) return;
  filEnCours = true;
  const demande = work;
  const coordonnees = new URLSearchParams({ work: demande, limit: '50' });
  if (suite && filCurseur) coordonnees.set('before', filCurseur);
  if ($('fil-decisions').checked) coordonnees.set('decisions', '1');
  try {
    const page = await api('/api/v1/activity?' + coordonnees);
    // Le travail a pu changer pendant la requête : ne pas mélanger deux fils.
    if (work !== demande) return;
    filTravail = demande;
    filEntrees = suite ? filEntrees.concat(page.entries) : page.entries;
    filCurseur = page.next_cursor || '';
    filSuite = !!page.more;
    filRendre();
  } catch (e) {
    $('fil-etat').textContent = 'Fil indisponible : ' + e.message;
  } finally {
    filEnCours = false;
  }
}

// Le rafraîchissement du cockpit ne doit pas replier les pages déjà demandées :
// on recharge autant d'entrées que l'opérateur en avait sous les yeux.
function filRafraichir() {
  if (!work) return;
  if (work !== filTravail) {
    filEntrees = [];
    filCurseur = '';
    filSuite = false;
    // La référence est prise une fois, à l'ouverture du travail.
    filVisiteReference = snapshot?.visit?.at || null;
    filCharger();
    return;
  }
  if (filEntrees.length <= 50) filCharger();
}

$('fil-plus').onclick = () => filCharger({ suite: true });
$('fil-decisions').onchange = () => {
  filEntrees = [];
  filCurseur = '';
  filCharger();
};

// Ce qui a changé depuis la visite de référence, d'après les entrées chargées.
// La réponse est bornée par la page en mémoire : si aucune entrée n'est
// antérieure à la visite, on ne peut pas affirmer un total, seulement un
// minimum. Mieux vaut le dire que compter faux.
function filChangementsDepuisVisite() {
  if (!filVisiteReference) return null;
  const taches = new Set();
  let depuis = 0, avant = 0;
  for (const e of filEntrees) {
    if (e.at >= filVisiteReference) {
      depuis++;
      if (e.task_id) taches.add(e.task_id);
    } else {
      avant++;
    }
  }
  return { taches: taches.size, entrees: depuis, complet: avant > 0 || !filSuite };
}
