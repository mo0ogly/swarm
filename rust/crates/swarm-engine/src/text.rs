//! strings.TrimSpace compatibility shared by engine validators.
/// Go Unicode whitespace classification shared by trimming and prose matching.
pub(crate) fn go_space(c: char) -> bool {
    matches!(c,'\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{0085}'|'\u{00a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}')
}
/// Trim exactly the Go whitespace set, retaining interior text and zero-width characters.
pub(crate) fn trim_space(value: &str) -> &str {
    value.trim_matches(go_space)
}

pub(crate) fn nonempty(value: &str) -> bool {
    !trim_space(value).is_empty()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trim_space_keeps_go_unicode_boundaries_and_original_interior() {
        for c in [
            '\t', '\n', '\u{000b}', '\u{000c}', '\r', ' ', '\u{0085}', '\u{00a0}', '\u{1680}',
            '\u{2000}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}',
        ] {
            assert!(!nonempty(&c.to_string()));
            assert_eq!(
                trim_space(&format!("{c}inside {c} text{c}")),
                format!("inside {c} text")
            );
        }
        for c in ['\0', '\u{001c}', '\u{200b}', '\u{feff}'] {
            assert!(nonempty(&c.to_string()));
        }
    }
}
