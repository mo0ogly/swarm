//! Terminal text compatibility: Go Unicode 15.0.0 Cc/Cf categories.
//! Generated from the installed Go unicode/tables.go, SHA256 80a109e5dd4ed40a85d69ddf95175f647bbd68c577d27eb475cda63790bb2487.
//! Category ranges/strides are representation data, not operator policy.
/// Preserve printable Unicode; LF/TAB become spaces and Cc/Cf disappear.
/// Input has already been decoded explicitly (invalid UTF-8 must be flagged by
/// its caller). This prevents terminal/bidi controls without erasing accents.
pub(crate) fn terminal_text(input: &str) -> String {
    input
        .chars()
        .filter_map(|c| {
            if matches!(c, '\n' | '\t') {
                return Some(' ');
            }
            let n = u32::from(c);
            if RANGES
                .iter()
                .any(|&(lo, hi, stride)| n >= lo && n <= hi && (n - lo) % stride == 0)
            {
                None
            } else {
                Some(c)
            }
        })
        .collect()
}
const RANGES: &[(u32, u32, u32)] = &[
    (0x0000, 0x001f, 1),
    (0x007f, 0x009f, 1),
    (0x00ad, 0x0600, 1363),
    (0x0601, 0x0605, 1),
    (0x061c, 0x06dd, 193),
    (0x070f, 0x0890, 385),
    (0x0891, 0x08e2, 81),
    (0x180e, 0x200b, 2045),
    (0x200c, 0x200f, 1),
    (0x202a, 0x202e, 1),
    (0x2060, 0x2064, 1),
    (0x2066, 0x206f, 1),
    (0xfeff, 0xfff9, 250),
    (0xfffa, 0xfffb, 1),
    (0x110bd, 0x110cd, 16),
    (0x13430, 0x1343f, 1),
    (0x1bca0, 0x1bca3, 1),
    (0x1d173, 0x1d17a, 1),
    (0xe0001, 0xe0020, 31),
    (0xe0021, 0xe007f, 1),
];
#[cfg(test)]
mod tests {
    /// Positive multilingual witness and controls include non-BMP Cf and gaps
    /// inside strided ranges, avoiding a broad range that erases ordinary text.
    #[test]
    fn terminal_categories_preserve_printable_text_and_strip_controls() {
        assert_eq!(
            super::terminal_text("é中🙂\n\t\r\x1b\u{202e}\u{e0001}a"),
            "é中🙂  a"
        );
        assert_eq!(
            super::terminal_text("\u{00ae}\u{0606}\u{110be}"),
            "\u{00ae}\u{0606}\u{110be}"
        );
    }
}
