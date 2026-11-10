//! Grapheme cluster state transitions, a faithful port of uniseg's
//! graphemerule.go (Unicode version 15.0.0).
//!
//! Licensing: derived from github.com/rivo/uniseg (MIT License),
//! which embeds Unicode data (Unicode, Inc. License).

use super::properties::{
    grapheme_property, PR_ANY, PR_CONTROL, PR_CR, PR_EXTEND, PR_EXTENDED_PICTOGRAPHIC, PR_L, PR_LF,
    PR_LV, PR_LVT, PR_PREPEND, PR_REGIONAL_INDICATOR, PR_SPACING_MARK, PR_T, PR_V, PR_ZWJ,
};

// Parser states (grAny = iota, ...).
pub(super) const ANY: i32 = 0;
const CR: i32 = 1;
const CONTROL_LF: i32 = 2;
const L: i32 = 3;
const LVV: i32 = 4;
const LVTT: i32 = 5;
const PREPEND: i32 = 6;
const EXTENDED_PICTOGRAPHIC: i32 = 7;
const EXTENDED_PICTOGRAPHIC_ZWJ: i32 = 8;
const RI_ODD: i32 = 9;
const RI_EVEN: i32 = 10;

const NO_BOUNDARY: i32 = 0;
const BOUNDARY: i32 = 1;

/// Port of `grTransitions`: maps (state, prop) to (newState, boundary
/// instruction, rule number). Returns (-1, -1, -1) if no transition is found.
fn gr_transitions(state: i32, prop: i32) -> (i32, i32, i32) {
    match (state, prop) {
        // GB5
        (ANY, PR_CR) => (CR, BOUNDARY, 50),
        (ANY, PR_LF) => (CONTROL_LF, BOUNDARY, 50),
        (ANY, PR_CONTROL) => (CONTROL_LF, BOUNDARY, 50),
        // GB4
        (CR, PR_ANY) => (ANY, BOUNDARY, 40),
        (CONTROL_LF, PR_ANY) => (ANY, BOUNDARY, 40),
        // GB3
        (CR, PR_LF) => (CONTROL_LF, NO_BOUNDARY, 30),
        // GB6
        (ANY, PR_L) => (L, BOUNDARY, 9990),
        (L, PR_L) => (L, NO_BOUNDARY, 60),
        (L, PR_V) => (LVV, NO_BOUNDARY, 60),
        (L, PR_LV) => (LVV, NO_BOUNDARY, 60),
        (L, PR_LVT) => (LVTT, NO_BOUNDARY, 60),
        // GB7
        (ANY, PR_LV) => (LVV, BOUNDARY, 9990),
        (ANY, PR_V) => (LVV, BOUNDARY, 9990),
        (LVV, PR_V) => (LVV, NO_BOUNDARY, 70),
        (LVV, PR_T) => (LVTT, NO_BOUNDARY, 70),
        // GB8
        (ANY, PR_LVT) => (LVTT, BOUNDARY, 9990),
        (ANY, PR_T) => (LVTT, BOUNDARY, 9990),
        (LVTT, PR_T) => (LVTT, NO_BOUNDARY, 80),
        // GB9
        (ANY, PR_EXTEND) => (ANY, NO_BOUNDARY, 90),
        (ANY, PR_ZWJ) => (ANY, NO_BOUNDARY, 90),
        // GB9a
        (ANY, PR_SPACING_MARK) => (ANY, NO_BOUNDARY, 91),
        // GB9b
        (ANY, PR_PREPEND) => (PREPEND, BOUNDARY, 9990),
        (PREPEND, PR_ANY) => (ANY, NO_BOUNDARY, 92),
        // GB11
        (ANY, PR_EXTENDED_PICTOGRAPHIC) => (EXTENDED_PICTOGRAPHIC, BOUNDARY, 9990),
        (EXTENDED_PICTOGRAPHIC, PR_EXTEND) => (EXTENDED_PICTOGRAPHIC, NO_BOUNDARY, 110),
        (EXTENDED_PICTOGRAPHIC, PR_ZWJ) => (EXTENDED_PICTOGRAPHIC_ZWJ, NO_BOUNDARY, 110),
        (EXTENDED_PICTOGRAPHIC_ZWJ, PR_EXTENDED_PICTOGRAPHIC) => {
            (EXTENDED_PICTOGRAPHIC, NO_BOUNDARY, 110)
        }
        // GB12 / GB13
        (ANY, PR_REGIONAL_INDICATOR) => (RI_ODD, BOUNDARY, 9990),
        (RI_ODD, PR_REGIONAL_INDICATOR) => (RI_EVEN, NO_BOUNDARY, 120),
        (RI_EVEN, PR_REGIONAL_INDICATOR) => (RI_ODD, BOUNDARY, 120),
        _ => (-1, -1, -1),
    }
}

/// Port of `transitionGraphemeState`. Given the current state and the next
/// code point, returns (newState, prop, boundary).
///
/// Fallback priority (from uniseg):
/// 1. specific state + specific property
/// 2. specific state + any property, and any state + specific property both
///    match: use state from (any state + prop), boundary from the lower rule
///    number (prefer the (any state + prop) transition on equal rules)
/// 3. only (state, any) matches
/// 4. only (any, prop) matches
/// 5. GB999: any ÷ any
pub(super) fn transition(state: i32, r: u32) -> (i32, i32, bool) {
    let prop = grapheme_property(r);

    let (next_state, next_prop, _) = gr_transitions(state, prop);
    if next_state >= 0 {
        return (next_state, prop, next_prop == BOUNDARY);
    }

    let (any_prop_state, any_prop_prop, any_prop_rule) = gr_transitions(state, PR_ANY);
    let (any_state_state, any_state_prop, any_state_rule) = gr_transitions(ANY, prop);

    if any_prop_state >= 0 && any_state_state >= 0 {
        let mut boundary = any_state_prop == BOUNDARY;
        if any_prop_rule < any_state_rule {
            boundary = any_prop_prop == BOUNDARY;
        }
        return (any_state_state, prop, boundary);
    }

    if any_prop_state >= 0 {
        return (any_prop_state, prop, any_prop_prop == BOUNDARY);
    }

    if any_state_state >= 0 {
        return (any_state_state, prop, any_state_prop == BOUNDARY);
    }

    // No known transition. GB999: Any ÷ Any.
    (ANY, prop, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crlf_is_no_boundary_gb3() {
        // CR followed by LF: GB3, no boundary, new state ControlLF.
        let (state, prop, boundary) = transition(ANY, 0x0d);
        assert_eq!((state, prop, boundary), (CR, PR_CR, true)); // boundary after CR
        let (state, prop, boundary) = transition(state, 0x0a);
        assert_eq!((state, prop, boundary), (CONTROL_LF, PR_LF, false));
    }

    #[test]
    fn hangul_l_v_no_boundary_gb6() {
        let (state, prop, boundary) = transition(ANY, 0x1100); // L
        assert_eq!((state, prop, boundary), (L, PR_L, true));
        let (state, prop, boundary) = transition(state, 0x1160); // V
        assert_eq!((state, prop, boundary), (LVV, PR_V, false));
    }

    #[test]
    fn fallback_lower_rule_wins() {
        // CR state + CR prop: no specific transition. Both (CR, ANY) [rule 40,
        // boundary] and (ANY, CR) [rule 50, boundary] match; lower rule (40) wins.
        let (state, prop, boundary) = transition(CR, 0x0d);
        assert_eq!((state, prop, boundary), (CR, PR_CR, true));

        // Prepend state + Extend prop: both (Prepend, ANY) rule 92 (no boundary)
        // and (ANY, Extend) rule 90 (no boundary) match; lower rule 90 wins,
        // state from the (ANY, prop) transition per spec.
        let (state, prop, boundary) = transition(PREPEND, 0x0301);
        assert_eq!((state, prop, boundary), (ANY, PR_EXTEND, false));
    }
}
