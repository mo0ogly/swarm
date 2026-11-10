//! Terminal-cell clipping matching the retained Go uniseg0.4.7 contract.
//! Uses its Unicode15 tables and state machine; no external Rust dependency,
//! execution policy, terminal detection or provider action. Upstream copyright
//! and MIT notice are retained in UNISEG-LICENSE.txt; generated data lists sources.
mod cluster;
mod grapheme;
mod properties;
use std::{error::Error, fmt};
/// Representation overflow, never a silent truncation or operator cutoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClipError {
    WidthOverflow,
}
impl fmt::Display for ClipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("terminal-cell-width-overflow")
    }
}
impl Error for ClipError {}
/// Measure full input using Go cluster widths; checked sums preserve overflow.
/// Pure, valid UTF8 input; this does not sanitize or infer terminal font behavior.
pub(crate) fn text_width(mut text: &str) -> Result<usize, ClipError> {
    let mut total = 0usize;
    let mut state = -1;
    while let Some(c) = cluster::first(text, state)? {
        total = total.checked_add(c.width).ok_or(ClipError::WidthOverflow)?;
        state = c.state;
        text = &text[c.end..];
    }
    Ok(total)
}
/// Longest whole-cluster prefix fitting supplied cells. Zero means empty.
/// No IO or policy/default; the only failure is width representation overflow.
fn truncate_cells(text: &str, width: usize) -> Result<&str, ClipError> {
    if width == 0 {
        return Ok("");
    }
    let mut rest = text;
    let mut end = 0usize;
    let mut cells = 0usize;
    let mut state = -1;
    while let Some(c) = cluster::first(rest, state)? {
        let next = cells.checked_add(c.width).ok_or(ClipError::WidthOverflow)?;
        if next > width {
            break;
        }
        cells = next;
        end += c.end;
        state = c.state;
        rest = &rest[c.end..];
    }
    Ok(&text[..end])
}
/// Exact Go clip: nonpositive width is empty; overflow uses whole clusters plus
/// ellipsis. Raw input is measured/truncated; only a full-fit return is sanitized,
/// preserving the Go quirk. Callers own sensitivity masking before this helper.
/// No IO, grants or defaults; representation overflow returns its typed cause.
pub(crate) fn clip(text: &str, width: isize) -> Result<String, ClipError> {
    if width < 1 {
        return Ok(String::new());
    }
    let width = usize::try_from(width).map_err(|_| ClipError::WidthOverflow)?;
    if text_width(text)? > width {
        if width < 2 {
            return Ok("…".into());
        }
        return Ok(format!("{}…", truncate_cells(text, width - 1)?));
    }
    Ok(crate::agent_terminal_text::terminal_text(text))
}
#[cfg(test)]
mod tests;
