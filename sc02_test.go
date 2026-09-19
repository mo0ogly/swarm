//go:build linux

package main

// SC-02 targeted unit tests: cell widths, sequence decoding, selection
// anchoring by identifier, paste handling.

import (
	"strings"
	"testing"
)

func TestTextWidthCellsNotRunes(t *testing.T) {
	// Latin: 1 cell per rune.
	if textWidth("abc") != 3 {
		t.Fatal("latin width")
	}
	// Wide CJK: 2 cells per rune (manifested via truncateCells behaviour).
	if w := textWidth("日本"); w < 4 {
		t.Fatalf("wide runes counted as %d cells", w)
	}
	// Combining mark adds no cell.
	if textWidth("é") != 1 {
		t.Fatal("combining mark counted")
	}
	// Control/format runes add no cell (already filtered upstream).
	if textWidth("a​b") != 2 {
		t.Fatalf("zero-width space: %d", textWidth("a​b"))
	}
}

func TestClipUsesCellWidth(t *testing.T) {
	// Wide characters occupy two cells: clipping must respect visual width.
	wide := "日本語" // 6 cells
	if got := clip(wide, 5); textWidth(got) > 5 {
		t.Fatalf("clip exceeded cell width: %q", got)
	}
	// Ellipsis keeps within width for latin text.
	if got := clip("abcdef", 4); got != "abc…" {
		t.Fatalf("latin clip: %q", got)
	}
	// Combining marks are not truncated away from their base letter.
	accented := "café" // visual 4 cells
	if got := clip(accented, 10); got != accented {
		t.Fatalf("accented text clipped: %q", got)
	}
}

func TestPadCellsAlignsColumns(t *testing.T) {
	a := padCells("日本", 8)
	b := padCells("abcd", 8)
	if textWidth(a) != 8 || textWidth(b) != 8 {
		t.Fatalf("padded widths: %d / %d", textWidth(a), textWidth(b))
	}
}

func TestReadableWrapRespectsCellWidth(t *testing.T) {
	rows := readableWrap("日本語のテキストです", 6)
	for i, row := range rows {
		if textWidth(row) > 6 {
			t.Fatalf("row %d exceeds 6 cells: %q (%d)", i, row, textWidth(row))
		}
	}
	if strings.Join(rows, "") != "日本語のテキストです" {
		t.Fatalf("content lost: %q", rows)
	}
}

func TestWrapDialogNeverLoops(t *testing.T) {
	rows := wrapDialog("日本語テキスト", 1)
	if len(rows) == 0 {
		t.Fatal("no output")
	}
	for _, row := range rows {
		if len([]rune(row)) > 1 {
			t.Fatalf("width 1 exceeded: %q", row)
		}
	}
}

func TestDecodeSequenceArrowsBothEncodings(t *testing.T) {
	cases := map[string]struct {
		seq  string
		want string
	}{
		"csi-up":     {"[A", "up"},
		"ss3-up":     {"OA", "up"},
		"csi-down":   {"[B", "down"},
		"ss3-down":   {"OB", "down"},
		"csi-right":  {"[C", "right"},
		"ss3-left":   {"OD", "left"},
		"paste-on":   {"[200~", "paste-start"},
		"paste-off":  {"[201~", "paste-end"},
		"delete-key": {"[3~", ""},
		"mouse-sgr":  {"[<0;33;5M", ""},
		"mouse-x10":  {"[M!!!", ""},
		"focus-out":  {"[O", ""},
		"home-ss3":   {"OH", ""},
	}
	for label, c := range cases {
		got, name := decodeSequence([]byte(c.seq), false)
		if !got {
			t.Fatalf("%s: sequence reported incomplete", label)
		}
		if name != c.want {
			t.Fatalf("%s: got %q want %q", label, name, c.want)
		}
	}
	if got, _ := decodeSequence([]byte("[1"), false); got {
		t.Fatal("partial CSI reported complete")
	}
}

func TestConsoleKeyByteHelpers(t *testing.T) {
	if dialogName('\t') != "tab" || dialogName(127) != "backspace" || dialogName(21) != "clear" {
		t.Fatal("dialog names")
	}
	in := []byte{}
	appendInput(&in, 'a')
	appendInput(&in, '\r')
	appendInput(&in, '\t')
	appendInput(&in, '\n')
	if string(in) != "a   " {
		t.Fatalf("controls not flattened: %q", in)
	}
	r := []byte{}
	appendRune(&r, 'é')
	if string(r) != "é" {
		t.Fatalf("rune append: %q", r)
	}
}

func TestSelectionAnchorFollowsIDNotIndex(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	c := &consoleState{}
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	frame := s.renderDashboard(w.ID, c, 100, 30, "")
	if !strings.Contains(frame, a.Provider) {
		t.Fatalf("agent row missing: %s", frame)
	}
	// Anchor on the agent like an operator Tab on the AGENTS panel.
	c.focus = 1
	s.renderDashboard(w.ID, c, 100, 30, "")
	if c.agentSelID != a.ID {
		t.Fatalf("agent selection not anchored: %q", c.agentSelID)
	}
	// Refresh must keep the same identifier, not drift by index.
	s.renderDashboard(w.ID, c, 100, 30, "")
	if c.agentSelID != a.ID {
		t.Fatalf("selection drifted after refresh: %q", c.agentSelID)
	}
	// Complete the old attempt before starting another on this task/workspace.
	code := 0
	if e = s.finishAgent(a, "completed", "Fixture complete", &code); e != nil {
		t.Fatal(e)
	}
	// A newer agent (rowid DESC puts it on top) must not steal the selection.
	w2, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	r2 := r
	r2.Revision = w2.Revision
	r2.EventID = newID("agent-")
	a2, _, e := s.prepare(w.ID, r2)
	if e != nil {
		t.Fatal(e)
	}
	if a2.ID == a.ID {
		t.Fatal("fixture produced identical agent ids")
	}
	// Same contract for the task panel: anchor on the work identifier.
	s.renderDashboard(w.ID, c, 100, 30, "")
	if c.agentSelID != a.ID {
		t.Fatal("new attempt stole selection")
	}
	current, _ := s.get(w.ID)
	current = applyTest(t, s, current, "task.add", Request{ID: "t2", Title: "Another", Deliverable: "proof", Criteria: []string{"proof"}, Owner: "test"})
	if e = s.priority(w.ID, "t2", 9); e != nil {
		t.Fatal(e)
	}
	c.taskSelID = r.TaskID
	c.focus = 0
	c.taskCursor = 0
	s.renderDashboard(w.ID, c, 100, 30, "")
	if c.taskSelID != r.TaskID {
		t.Fatalf("task selection drifted after refresh: %q", c.taskSelID)
	}
}

func TestPasteMarkersToggleAndFlatText(t *testing.T) {
	got, name := decodeSequence([]byte("[200~"), false)
	if !got || name != "paste-start" {
		t.Fatalf("paste start: %q", name)
	}
	in := []byte{}
	for _, k := range []byte{'s', '\r', 't', '\n', 'a', '\t', 't'} {
		appendInput(&in, k)
	}
	// "s" then flattened controls then "ta t": no command executed mid-paste.
	if strings.Contains(string(in), "\r") || strings.Contains(string(in), "\n") {
		t.Fatalf("control byte kept in paste: %q", in)
	}
}
