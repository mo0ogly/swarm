// terminal_clip/cluster.rs
//
// Port of uniseg's FirstGraphemeClusterInString / runeWidth (github.com/rivo/uniseg, MIT).
// Unicode data tables: Unicode 15.0 (uniseg-derived). Keep this attribution.

use super::grapheme::{transition, ANY};
use super::properties::{
    east_asian_property, emoji_presentation, grapheme_property, PR_A, PR_CONTROL, PR_CR,
    PR_EMOJI_PRESENTATION, PR_EXTEND, PR_EXTENDED_PICTOGRAPHIC, PR_F, PR_L, PR_LF, PR_W, PR_ZWJ,
};
use super::ClipError;

/// Variation Selector-15 (text presentation).
const VS15: u32 = 0xfe0e;
/// Variation Selector-16 (emoji presentation).
const VS16: u32 = 0xfe0f;

/// Go uniseg protocol default: ambiguous-width code points use one cell.
/// This is a deterministic JSON presentation convention, not an execution policy.
pub(super) const EAST_ASIAN_AMBIGUOUS_WIDTH: usize = 1;

const SHIFT_GRAPHEME_PROP_STATE: i32 = 4;
const MASK_GRAPHEME_STATE: i32 = (1 << SHIFT_GRAPHEME_PROP_STATE) - 1;

/// The first grapheme cluster of `text`, its width in cells, and the parser
/// state to resume with. `state < 0` means "unknown" (fresh start).
///
/// `Cluster::end` is a UTF-8 byte offset into `text`.
pub(super) fn first(text: &str, state: i32) -> Result<Option<Cluster>, ClipError> {
    let mut chars = text.char_indices();
    let Some((_, first)) = chars.next() else {
        return Ok(None);
    };
    let r = first as u32;
    let first_len = first.len_utf8();
    if text.len() == first_len {
        let prop = if state < 0 {
            grapheme_property(r)
        } else {
            state >> SHIFT_GRAPHEME_PROP_STATE
        };
        return Ok(Some(Cluster {
            end: first_len,
            width: rune_width(r, prop),
            state: ANY | (prop << SHIFT_GRAPHEME_PROP_STATE),
        }));
    }
    let (mut state, first_prop) = if state < 0 {
        let (next, prop, _) = transition(state, r);
        (next, prop)
    } else {
        (state, state >> SHIFT_GRAPHEME_PROP_STATE)
    };
    let mut width = rune_width(r, first_prop);
    let mut last_prop = first_prop;
    for (offset, next) in chars {
        let r = next as u32;
        let (new_state, prop, boundary) = transition(state & MASK_GRAPHEME_STATE, r);
        state = new_state;
        if boundary {
            return Ok(Some(Cluster {
                end: offset,
                width,
                state: state | (prop << SHIFT_GRAPHEME_PROP_STATE),
            }));
        }
        if first_prop == PR_EXTENDED_PICTOGRAPHIC {
            if r == VS15 {
                width = 1;
            } else if r == VS16 {
                width = 2;
            }
        } else if first_prop != PR_REGIONAL_INDICATOR && first_prop != PR_L {
            width = width
                .checked_add(rune_width(r, prop))
                .ok_or(ClipError::WidthOverflow)?;
        }
        last_prop = prop;
    }
    Ok(Some(Cluster {
        end: text.len(),
        width,
        state: ANY | (last_prop << SHIFT_GRAPHEME_PROP_STATE),
    }))
}

use super::properties::PR_REGIONAL_INDICATOR;

/// A grapheme cluster: byte offset of its end within the input, its display
/// width in cells, and the parser state (property in the high bits, exact
/// uniseg grapheme state in the low 4 bits).
pub(super) struct Cluster {
    pub(super) end: usize,
    pub(super) width: usize,
    pub(super) state: i32,
}

/// Monospace width of a rune given its grapheme property.
/// Exact port of uniseg's `runeWidth`.
pub(super) fn rune_width(r: u32, grapheme_property_value: i32) -> usize {
    match grapheme_property_value {
        PR_CONTROL | PR_CR | PR_LF | PR_EXTEND | PR_ZWJ => return 0,
        PR_REGIONAL_INDICATOR => return 2,
        PR_EXTENDED_PICTOGRAPHIC => {
            if emoji_presentation(r) == PR_EMOJI_PRESENTATION {
                return 2;
            }
            return 1;
        }
        _ => {}
    }

    match r {
        0x2e3a => return 3, // TWO-EM DASH
        0x2e3b => return 4, // THREE-EM DASH
        _ => {}
    }

    match east_asian_property(r) {
        PR_W | PR_F => return 2,
        PR_A => return EAST_ASIAN_AMBIGUOUS_WIDTH,
        _ => {}
    }

    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_cluster_is_one_cell() {
        let c = first("ab", -1).unwrap().unwrap();
        assert_eq!(c.end, 1);
        assert_eq!(c.width, 1);
    }

    #[test]
    fn emoji_with_vs16_is_two_cells() {
        // U+2764 HEAVY BLACK HEART + VS16 (extended pictographic, emoji presentation forced).
        let text = "\u{2764}\u{fe0f}x";
        let c = first(text, -1).unwrap().unwrap();
        assert_eq!(c.end, 6); // 3 + 3 bytes
        assert_eq!(c.width, 2);
    }

    #[test]
    fn regional_indicators_pair_as_one_cluster() {
        // Two regional indicators form one cluster of width 2.
        let text = "\u{1f1e9}\u{1f1ea}ok";
        let c = first(text, -1).unwrap().unwrap();
        assert_eq!(c.end, 8); // 2 * 4 bytes
        assert_eq!(c.width, 2);
    }
}
