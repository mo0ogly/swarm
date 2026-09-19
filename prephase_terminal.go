//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"strconv"
	"strings"
)

const preparationTerminalHelp = `PRÉPARER — conversation et documents partagés avec le web
Texte libre : envoyer à l’IA sélectionnée. Sans préparation : enregistrer le besoin.
/ia [nom] : lister ou choisir l’IA ; /plan [consigne] : proposer le plan
/voir besoin|brief|plan ; /besoin TEXTE ; /brief TEXTE
/multiligne [message|besoin|brief] ; terminer par /envoyer ou /annuler
/edit besoin|brief|plan : ouvrir VISUAL ou EDITOR, sans shell
/appliquer ID : comparer une proposition ; /confirmer : l’enregistrer
/creer-missions : relire et créer ; /autoriser-missions : autoriser les départs
/decisions : lister ; /repondre NUMÉRO TEXTE : enregistrer une réponse
/adopter : relire le brief avant adoption ; /verifier : vérifier le plan
/arreter : demander l’arrêt ; /reprendre : nouvelle tentative explicite ; /historique : relire ; /renvoyer : vérifier un envoi
/sessions ; /ouvrir ID ; /nouveau TITRE ; /actualiser ; /methode [nom]
/contexte full|recent : choisir la conversation complète ou le dernier échange
/fichiers [recherche] ; /fichiers-dans DOSSIER ; /lire CHEMIN DEBUT FIN ; /joindre CHEMIN DEBUT FIN
/retirer CHEMIN ; /budget [LIMITE RESERVE DATE SOURCE] ; /sources : extraits transmis ; /export DOSSIER ; /aide ; /quitter
Un seul appel actif, au plus 120 s, 20 échanges. Aucun lancement de mission.
Ctrl-C abandonne la saisie ou demande l’arrêt. EOF ferme la vue, pas la génération.`

type preparationTerminal struct {
	contextMode  string
	s            *Store
	p            Preparation
	provider     *PreparationCapability
	out          io.Writer
	width        int
	pending      *PreparationSend
	confirmation *PreparationRequest
	multi        string
	lines        []string
	seen         map[string]string
	editor       func(string) error
}

func preparationDisplayDocument(kind, text string) string {
	if kind != "plan" {
		return text
	}
	var value any
	if json.Unmarshal([]byte(text), &value) != nil {
		return text
	}
	b, e := json.MarshalIndent(value, "", "  ")
	if e != nil {
		return text
	}
	return string(b)
}
func (t *preparationTerminal) say(text string) {
	for _, line := range strings.Split(text, "\n") {
		if line == "" {
			fmt.Fprintln(t.out)
			continue
		}
		for _, row := range readableWrap(line, t.width) {
			fmt.Fprintln(t.out, row)
		}
	}
}
func (t *preparationTerminal) prompt() {
	if t.confirmation != nil {
		fmt.Fprint(t.out, "/confirmer ou /annuler > ")
		return
	}
	if t.multi != "" {
		fmt.Fprint(t.out, "... ")
		return
	}
	fmt.Fprint(t.out, "préparer> ")
}
func (t *preparationTerminal) load(id string) error {
	p, e := t.s.preparation(id)
	if e != nil {
		return e
	}
	t.p = p
	t.seen = map[string]string{}
	t.say(fmt.Sprintf("%s · %s · révision %d enregistrée · méthode %s · document %s", p.Title, p.ID, p.Revision, p.Method, preparationCurrentDocument(p)))
	return nil
}
func (t *preparationTerminal) mutate(r PreparationRequest) error {
	p, e := t.s.preparationCommand(r)
	if e != nil {
		return e
	}
	if p.ReceiptHistorical {
		t.say("Commande déjà enregistrée ; chargement de l’état courant.")
		return t.load(p.ID)
	}
	t.p = p
	if p.Conversion != nil {
		t.say(fmt.Sprintf("Missions : %s — travail %s. Départs autorisés : %t.", strings.Join(p.Conversion.TaskIDs, ", "), p.Conversion.WorkID, p.Conversion.ReleasedAt != ""))
	}
	t.say(fmt.Sprintf("Enregistré · révision %d · méthode %s · document %s.", p.Revision, p.Method, preparationCurrentDocument(p)))
	return nil
}

func preparationCurrentDocument(p Preparation) string {
	for _, kind := range []string{"plan", "brief", "besoin"} {
		if p.Documents[kind].Text != "" {
			return kind
		}
	}
	return "besoin"
}
func (t *preparationTerminal) request(action string) PreparationRequest {
	revision := t.p.Revision
	return PreparationRequest{Version: 1, ID: t.p.ID, Event: newID("terminal-"), Revision: &revision, Action: action}
}
func (t *preparationTerminal) create(title, text string) error {
	r := t.request("create")
	r.ID = ""
	zero := 0
	r.Revision = &zero
	r.Title = title
	r.Text = text
	if e := t.mutate(r); e != nil {
		return e
	}
	t.say("Préparation : " + t.p.ID + ". Choisissez /ia NOM pour dialoguer.")
	return nil
}
func (t *preparationTerminal) send(message, target string) error {
	if t.pending != nil {
		return fmt.Errorf("Envoi non confirmé : /renvoyer vérifie le même message.")
	}
	if t.p.ID == "" {
		if target == "plan" {
			return fmt.Errorf("Créez un besoin, puis adoptez le brief.")
		}
		return t.create("Nouvelle préparation", message)
	}
	if t.provider == nil {
		return fmt.Errorf("Choisissez une IA avec /ia NOM avant l’envoi.")
	}
	t.pending = &PreparationSend{ContextMode: t.contextMode, Version: 1, ID: t.p.ID, Event: newID("terminal-send-"), Revision: t.p.Revision, Provider: t.provider.Provider, Capability: t.provider.Hash, Message: message, Target: target}
	return t.resend()
}
func (t *preparationTerminal) resend() error {
	if t.pending == nil {
		return fmt.Errorf("Aucun envoi à vérifier.")
	}
	turn, e := t.s.sendPreparation(*t.pending)
	if e != nil {
		if _, known := e.(*PreparationError); known {
			t.pending = nil
		}
		return e
	}
	if e = t.s.spawnPreparationTurn(turn); e != nil {
		return fmt.Errorf("Envoi non confirmé : /renvoyer. %w", e)
	}
	t.pending = nil
	t.say("Envoi enregistré : " + turn.ID + ". /arreter reste disponible.")
	return nil
}
func (t *preparationTerminal) history(all bool) error {
	if t.p.ID == "" {
		return nil
	}
	ts, e := t.s.preparationDialogue(t.p.ID)
	if e != nil {
		return e
	}
	if t.seen == nil {
		t.seen = map[string]string{}
	}
	for _, turn := range ts {
		if !all && t.seen[turn.ID] == turn.Status {
			continue
		}
		t.seen[turn.ID] = turn.Status
		t.say(turn.ID + " · " + turn.Provider + " · " + turn.Status)
		if all {
			t.say("Vous : " + turn.Question)
		}
		if turn.Answer != nil {
			t.say("IA : " + turn.Answer.Message)
			kind, text := turn.proposalDocument()
			if text != "" {
				t.say("Proposition de " + kind + " : /appliquer " + turn.ID)
			}
		}
		if turn.PlanSummary != nil {
			c := turn.PlanSummary
			t.say(fmt.Sprintf("%d missions · %d dépendances · %d questions ouvertes dans la proposition IA.", c.Missions, c.Dependencies, c.OpenQuestions))
		}
		if turn.Stale && !turn.Used && !turn.UsedInPlan {
			t.say("Contexte ancien : demandez une proposition actualisée.")
		}
		if turn.Error != "" {
			t.say(turn.Error)
		}
	}
	return nil
}
func (t *preparationTerminal) stop() error {
	ts, e := t.s.preparationDialogue(t.p.ID)
	if e != nil {
		return e
	}
	for _, turn := range ts {
		if turn.active() {
			current, e := t.s.cancelPreparation(t.p.ID, turn.ID)
			if e != nil {
				return e
			}
			t.say("Arrêt : " + current.Status + ". Aucune relance automatique.")
			return nil
		}
	}
	t.say("Aucun échange actif.")
	return nil
}

// resume starts a distinct attempt from the last interrupted or failed turn.
// It never revives a process and never reuses the prior idempotency event.
func (t *preparationTerminal) resume() error {
	if t.p.ID == "" {
		return fmt.Errorf("Ouvrez d’abord une préparation.")
	}
	ts, e := t.s.preparationDialogue(t.p.ID)
	if e != nil {
		return e
	}
	if len(ts) == 0 {
		return fmt.Errorf("Aucun échange arrêté à reprendre.")
	}
	last := ts[len(ts)-1]
	if last.active() {
		return preparationError("interrupted", "Arrêt non confirmé ; attendez son état interrompu avant /reprendre.")
	}
	if last.Status != "interrupted" && last.Status != "failed" {
		return fmt.Errorf("Le dernier échange est %s ; /reprendre est réservé à un arrêt ou un échec.", last.Status)
	}
	if t.provider == nil || t.provider.Provider != last.Provider {
		return fmt.Errorf("Sélectionnez explicitement /ia %s avant de reprendre.", last.Provider)
	}
	t.contextMode = last.ContextMode
	return t.send(last.Question, last.Target)
}

// Commands are only interpreted when typed, never from a bracketed paste.
func (t *preparationTerminal) line(line string, literal bool) (bool, error) {
	if t.multi != "" {
		if !literal && line == "/annuler" {
			t.multi = ""
			t.lines = nil
			t.say("Saisie abandonnée.")
			return false, nil
		}
		if !literal && line == "/envoyer" {
			text := strings.Join(t.lines, "\n")
			kind := t.multi
			t.multi = ""
			t.lines = nil
			if kind == "message" {
				return false, t.send(text, "")
			}
			return false, t.saveDocument(kind, text)
		}
		if len(strings.Join(t.lines, "\n"))+len(line) > 16000 {
			return false, fmt.Errorf("Saisie trop longue ; /annuler ou /envoyer le texte déjà saisi.")
		}
		t.lines = append(t.lines, line)
		return false, nil
	}
	if t.confirmation != nil {
		if literal {
			return false, fmt.Errorf("Tapez /confirmer ou /annuler ; le collage ne confirme pas.")
		}
		if line == "/annuler" {
			t.confirmation = nil
			t.say("Action abandonnée.")
			return false, nil
		}
		if line != "/confirmer" {
			return false, fmt.Errorf("Relisez la comparaison puis /confirmer ou /annuler.")
		}
		r := *t.confirmation
		t.confirmation = nil
		return false, t.mutate(r)
	}
	if strings.TrimSpace(line) == "" {
		return false, nil
	}
	if literal || !strings.HasPrefix(line, "/") {
		return false, t.send(line, "")
	}
	cmd, arg, _ := strings.Cut(line, " ")
	arg = strings.TrimSpace(arg)
	switch cmd {
	case "/quitter":
		return true, nil
	case "/aide":
		if arg != "" {
			text, err := cliTopicHelp(arg)
			if err != nil {
				return false, err
			}
			t.say(text)
		} else {
			t.say(preparationTerminalHelp + "\nAide par sujet : /aide preparation | /aide contexte | /aide budget | /aide parite")
		}
	case "/sessions":
		ps, e := t.s.preparations()
		if e != nil {
			return false, e
		}
		for _, p := range ps {
			t.say(p.ID + " · " + p.Title)
		}
	case "/ouvrir":
		if t.pending != nil {
			return false, fmt.Errorf("Vérifiez l’envoi avec /renvoyer avant de changer de préparation.")
		}
		return false, t.load(arg)
	case "/nouveau":
		if t.pending != nil {
			return false, fmt.Errorf("Vérifiez l’envoi avec /renvoyer.")
		}
		return false, t.create(arg, "")
	case "/actualiser":
		return false, t.load(t.p.ID)
	case "/ia":
		capabilities := t.s.preparationCapabilities()
		if len(capabilities) == 0 {
			return false, fmt.Errorf("Aucune IA configurée. Configurez les fournisseurs Swarm puis utilisez /ia ; les documents restent éditables.")
		}
		for _, c := range capabilities {
			if arg == "" {
				t.say(fmt.Sprintf("%s · disponible %t · délai %d s · %s", c.Provider, c.Available, c.Timeout, c.Reason))
			}
			if c.Provider == arg {
				if !c.Available {
					return false, fmt.Errorf("%s", c.Reason)
				}
				t.provider = &c
				t.say("IA sélectionnée : " + c.Provider + ". Documents, méthode et conversation transmis ; aucun outil.")
				return false, nil
			}
		}
		if arg != "" {
			return false, fmt.Errorf("IA inconnue : /ia liste les choix.")
		}
	case "/contexte":
		if t.pending != nil {
			return false, fmt.Errorf("Vérifiez l’envoi en attente avant de changer son contexte.")
		}
		if arg != "full" && arg != "recent" {
			return false, fmt.Errorf("/contexte full|recent")
		}
		t.contextMode = arg
		if arg == "recent" {
			t.say("Le prochain appel recevra les documents, la méthode et le dernier échange complet. Les échanges antérieurs restent dans l’historique, sans être renvoyés.")
		} else {
			t.say("Le prochain appel recevra la conversation complète.")
		}
	case "/fichiers", "/fichiers-dans":
		scope, query := "", arg
		if cmd == "/fichiers-dans" {
			scope, query = arg, ""
		}
		v, e := t.s.preparationSourceFiles(scope, query)
		if e != nil {
			return false, e
		}
		for _, dir := range v.Directories {
			t.say("Dossier : " + dir + " — /fichiers-dans " + dir)
		}
		for _, path := range v.Paths {
			t.say(path)
		}
		if v.Limited {
			t.say("Liste limitée : précisez la recherche.")
		}
	case "/lire", "/joindre":
		parts := strings.Fields(arg)
		if len(parts) < 3 {
			return false, fmt.Errorf("%s CHEMIN DEBUT FIN", cmd)
		}
		start, e := strconv.Atoi(parts[len(parts)-2])
		if e != nil {
			return false, e
		}
		end, e := strconv.Atoi(parts[len(parts)-1])
		if e != nil {
			return false, e
		}
		path := strings.Join(parts[:len(parts)-2], " ")
		v, e := t.s.preparationReadSource(path, start, end)
		if e != nil {
			return false, e
		}
		t.say(fmt.Sprintf("%s · lignes %d–%d · %s\n%s", v.Path, v.Start, v.End, v.Hash, v.Text))
		if cmd == "/joindre" {
			r := t.request("source-add")
			r.Source = &v.PreparationSourceRef
			t.confirmation = &r
			t.say("/confirmer joint cet instantané aux prochains appels et demande de réadopter le brief ; /annuler abandonne.")
		}
	case "/retirer":
		r := t.request("source-remove")
		r.Source = &PreparationSourceRef{Path: arg}
		t.confirmation = &r
		t.say("Retirer " + arg + " des prochains appels : /confirmer ou /annuler.")
	case "/budget":
		if arg == "" {
			v, e := t.s.preparationBudget(t.p.ID)
			if e != nil {
				return false, e
			}
			b, _ := json.MarshalIndent(v, "", "  ")
			t.say(string(b))
		} else {
			parts := strings.Fields(arg)
			if len(parts) < 4 {
				return false, fmt.Errorf("/budget LIMITE_USD RESERVE_USD AAAA-MM-JJ SOURCE_ESTIMATION")
			}
			limit, e := strconv.ParseFloat(parts[0], 64)
			if e != nil {
				return false, e
			}
			reserve, e := strconv.ParseFloat(parts[1], 64)
			if e != nil {
				return false, e
			}
			b := Budget{Limit: limit, Reserve: reserve, PriceDate: parts[2], Source: strings.Join(parts[3:], " ")}
			if e = validateBudget(b); e != nil {
				return false, e
			}
			r := t.request("budget")
			r.Budget = &b
			t.confirmation = &r
			t.say(fmt.Sprintf("Enveloppe estimative %.2f USD ; %.2f USD par appel. /confirmer ou /annuler.", limit, reserve))
		}
	case "/sources":
		for _, source := range t.p.Sources {
			t.say(fmt.Sprintf("%s · lignes %d–%d · instantané %s", source.Path, source.Start, source.End, source.At))
		}

		t.say("Documents enregistrés, extraits joints, brief adopté, conversation et fichiers de la méthode. Aucun accès autonome au dépôt.")
		m, e := t.s.preparationMethod(t.p.Method)
		if e != nil {
			return false, e
		}
		for _, p := range m.Paths {
			t.say(p)
		}
	case "/methode":
		if arg == "" {
			for _, m := range t.s.preparationMethods() {
				t.say(fmt.Sprintf("%s · disponible %t · %s", m.ID, m.Available, m.Reason))
			}
		} else {
			r := t.request("method")
			r.Method = arg
			return false, t.mutate(r)
		}
	case "/voir":
		if !preparationDocumentKind(arg) {
			return false, fmt.Errorf("/voir besoin|brief|plan")
		}
		t.say(preparationDisplayDocument(arg, t.p.Documents[arg].Text))
	case "/besoin":
		return false, t.saveDocument("besoin", arg)
	case "/brief":
		return false, t.saveDocument("brief", arg)
	case "/multiligne":
		if arg == "" {
			arg = "message"
		}
		if arg != "message" && arg != "besoin" && arg != "brief" {
			return false, fmt.Errorf("/multiligne message|besoin|brief")
		}
		t.multi = arg
		t.lines = nil
		t.say("Saisie multiligne ; /envoyer termine, /annuler abandonne.")
	case "/plan":
		if arg == "" {
			arg = "Propose le plan JSON à partir du brief adopté."
		}
		return false, t.send(arg, "plan")
	case "/historique":
		return false, t.history(true)
	case "/arreter":
		return false, t.stop()
	case "/reprendre":
		return false, t.resume()
	case "/renvoyer":
		return false, t.resend()
	case "/appliquer":
		turn, e := t.s.preparationTurn(arg)
		if e != nil {
			return false, e
		}
		kind, text := turn.proposalDocument()
		if turn.PreparationID != t.p.ID || turn.Status != "answered" || turn.Revision != t.p.Revision || text == "" {
			return false, fmt.Errorf("Proposition absente, ancienne ou hors préparation.")
		}
		t.say("DOCUMENT ENREGISTRÉ — " + kind)
		t.say(preparationDisplayDocument(kind, t.p.Documents[kind].Text))
		t.say("PROPOSITION — " + kind)
		t.say(preparationDisplayDocument(kind, text))
		r := t.request("use-proposal")
		r.Turn = turn.ID
		t.confirmation = &r
		t.say("/confirmer remplace ce document ; sa vérification ou adoption reste distincte.")
	case "/adopter":
		if t.p.Documents["brief"].Text == "" {
			return false, fmt.Errorf("Rédigez d’abord un brief.")
		}
		t.say(t.p.Documents["brief"].Text)
		r := t.request("adopt-brief")
		r.Hash = t.p.Documents["brief"].Hash
		t.confirmation = &r
		t.say("/confirmer adopte ce brief.")
	case "/creer-missions", "/autoriser-missions":
		review, e := t.s.preparationConversionReview(t.p.ID)
		if e != nil {
			return false, e
		}
		if review.Preparation.Revision != t.p.Revision {
			return false, fmt.Errorf("Préparation modifiée : /actualiser avant de relire les missions.")
		}
		action := "create-missions"
		planHash := t.p.Documents["plan"].Hash
		if cmd == "/autoriser-missions" {
			action = "release-plan"
			if t.p.Conversion == nil {
				return false, fmt.Errorf("Créer les missions avant d’autoriser leurs départs.")
			}
			planHash = t.p.Conversion.PlanHash
		}
		t.say("Travail cible : " + review.WorkTitle)
		for _, m := range review.Spec.Tasks {
			t.say(fmt.Sprintf("%s — %s ; dépendances : %s ; livrable : %s", m.ID, m.Title, strings.Join(m.Depends, ", "), m.Deliverable))
		}
		r := t.request(action)
		r.Hash = planHash
		r.WorkRevision = &review.WorkRevision
		t.confirmation = &r
		if action == "create-missions" {
			t.say("/confirmer crée ces missions avec leur démarrage verrouillé.")
		} else {
			t.say("/confirmer autorise ces seules missions. Le moteur conserve profils, dépendances, budget, pause et autonomie ; en mode automatique, les missions éligibles pourront démarrer.")
		}
	case "/decisions", "/repondre":
		var plan ActionPlan
		if e := strict([]byte(t.p.Documents["plan"].Text), &plan); e != nil {
			return false, fmt.Errorf("Enregistrez d’abord un plan JSON valide : %w", e)
		}
		if _, e := validateActionPlan(plan, false); e != nil {
			return false, e
		}
		if cmd == "/repondre" {
			number, answer, _ := strings.Cut(arg, " ")
			i, e := strconv.Atoi(number)
			if e != nil || i < 1 || i > len(plan.Questions) || !nonempty(answer) {
				return false, fmt.Errorf("/repondre NUMÉRO TEXTE — choisir un numéro affiché par /decisions.")
			}
			plan.Questions[i-1].Answer = answer
			r := t.request("answer-questions")
			r.Hash = t.p.Documents["plan"].Hash
			r.Decisions = plan.Questions
			if e := t.mutate(r); e != nil {
				return false, e
			}
			t.say("Réponse enregistrée. /verifier contrôle le plan une fois toutes les décisions résolues.")
		}
		if len(plan.Questions) == 0 {
			t.say("Aucune décision ouverte dans ce plan. /verifier pour le contrôler.")
		}
		for i, q := range plan.Questions {
			answer := q.Answer
			if !nonempty(answer) {
				answer = "À RÉSOUDRE"
			}
			t.say(fmt.Sprintf("%d. %s\n   %s", i+1, q.Question, answer))
		}
	case "/verifier":
		r := t.request("validate-plan")
		r.Hash = t.p.Documents["plan"].Hash
		if e := t.mutate(r); e != nil {
			return false, e
		}
		t.say("Plan vérifié pour cette version. Aucune mission créée.")
	case "/export":
		if e := exportPreparation(t.p, arg); e != nil {
			return false, e
		}
		t.say("Documents exportés : " + arg)
	case "/edit":
		if !preparationDocumentKind(arg) {
			return false, fmt.Errorf("/edit besoin|brief|plan")
		}
		if t.editor == nil {
			return false, fmt.Errorf("Éditeur indisponible ici.")
		}
		return false, t.editor(arg)
	default:
		return false, fmt.Errorf("Commande inconnue. /aide ; aucune commande shell exécutée.")
	}
	return false, nil
}
func (t *preparationTerminal) saveDocument(kind, text string) error {
	if t.p.ID == "" && kind == "besoin" {
		return t.create("Nouvelle préparation", text)
	}
	r := t.request("save")
	r.Document = kind
	r.Text = text
	return t.mutate(r)
}
