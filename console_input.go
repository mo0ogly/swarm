//go:build linux

package main

import (
	"unicode/utf8"
)

func dialogTypeText(d *taskDialog, target *string, key byte) {
	if len(*target) >= 8000 {
		return
	}
	switch key {
	case '\r', '\n', '\t':
		*target += " "
	case 127, 8:
		if len(*target) > 0 {
			_, n := utf8.DecodeLastRuneInString(*target)
			*target = (*target)[:len(*target)-n]
		}
	default:
		if key >= 32 {
			*target += string([]byte{key})
		}
	}
}

// appendInput appends a byte as text, flattening line and tab controls so a
// paste can not execute an intermediate command or switch panel focus.
func appendInput(input *[]byte, key byte) {
	if len(*input) >= 16000 {
		return
	}
	if key == '\r' || key == '\n' {
		*input = append(*input, ' ')
		return
	}
	if key == '\t' {
		*input = append(*input, ' ')
		return
	}
	if key >= 32 {
		*input = append(*input, key)
	}
}

func appendRune(input *[]byte, r rune) {
	if len(*input) >= 16000 {
		return
	}
	*input = utf8.AppendRune(*input, r)
}

// dialogName maps a single control byte to the dialog key vocabulary.
func dialogName(key byte) string {
	switch key {
	case '\t':
		return "tab"
	case '\r', '\n':
		return "enter"
	case 127, 8:
		return "backspace"
	case 21:
		return "clear"
	}
	if key >= 32 {
		return "text:" + string([]byte{key})
	}
	return ""
}

// decodeSequence classifies a completed or partial escape sequence.
// It returns terminal=true when the sequence is complete, and a key name:
// arrows, tab/enter/backspace/clear, paste markers, or "text:" payload.
// Unknown sequences (mouse reports, edit keys) return terminal=true with an
// empty name so they are ignored rather than typed into the command line.
func decodeSequence(seq []byte, paste bool) (bool, string) {
	if len(seq) < 2 {
		return false, ""
	}
	if seq[0] == 'O' {
		// SS3: complete on the single following byte.
		if len(seq) < 2 {
			return false, ""
		}
		switch seq[1] {
		case 'A':
			return true, "up"
		case 'B':
			return true, "down"
		case 'C':
			return true, "right"
		case 'D':
			return true, "left"
		case 'P':
			return true, "help"
		case 'H':
			return true, ""
		case 'F':
			return true, ""
		}
		return true, ""
	}
	if len(seq) >= 2 && seq[0] == '[' && seq[1] == 'M' {
		return len(seq) >= 5, ""
	}
	if seq[0] != '[' {
		return true, ""
	}
	final := seq[len(seq)-1]
	if final < 0x40 || final > 0x7E {
		if len(seq) > 32 {
			return true, "" // runaway sequence: drop it
		}
		return false, ""
	}
	params := string(seq[1 : len(seq)-1])
	switch final {
	case 'A':
		return true, "up"
	case 'B':
		return true, "down"
	case 'C':
		return true, "right"
	case 'D':
		return true, "left"
	case '~':
		// Edit keys and paste markers, identified by parameters.
		switch params {
		case "11":
			return true, "help"
		case "200":
			return true, "paste-start"
		case "201":
			return true, "paste-end"
		}
		return true, ""
	}
	// Mouse reports (<0;…M/m) and other CSI finals: ignore.
	return true, ""
}
