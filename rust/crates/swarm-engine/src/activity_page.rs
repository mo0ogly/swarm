//! Pure activity DTO/cursor/pagination used by the native reader. Operator page
//! and source policies are supplied by the caller; this module grants no rights.
#![forbid(unsafe_code)]

use serde::Serialize;

/// Origin value the Go producer writes for a decision made by a human
/// (« vous »). `decisions_only` keeps strictly this value; no other string
/// and no inference from `kind` is applied, because classification belongs
/// to the producer, not to this module.
pub const ACTIVITY_ORIGIN_HUMAN: &str = "vous";

/// The two producer tables behind the feed, named as the Go engine names
/// them in its technical cursor protocol: `c` for the cockpit-events table
/// (ordered by `seq`) and `e` for the work-events table (ordered by
/// `revision`).
///
/// This enum names write sources for cursor formatting only. It does not
/// classify event kinds, does not decide origins, and grants no permission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivitySource {
    /// Cockpit events table, Go cursor prefix `c`.
    Cockpit,
    /// Work events table, Go cursor prefix `e`.
    WorkEvent,
}

/// Formats the rank component of a pagination cursor: the Go engine's
/// technical protocol `prefix + %012d` (e.g. `c000000000007`,
/// `e000000000042`).
///
/// Pure formatting: no I/O, no clock, no validation of where `sequence`
/// came from — its provenance (a rowid or a revision) is the producer's
/// responsibility, and this function deliberately does not police it.
///
/// Signed values are formatted exactly as Go's `%012d` does: the sign
/// counts inside the 12-character width (`-5` → `c-00000000005`). Ordering
/// caveat, stated rather than assumed: zero padding preserves numeric order
/// under byte comparison only for non-negative sequences whose digits fit
/// the width; across signs, and for magnitudes wider than the field, byte
/// order no longer tracks numeric order — exactly as in Go. This module
/// asserts nothing beyond that width and does not reject wider values.
pub fn activity_rank(source: ActivitySource, sequence: i64) -> String {
    let prefix = match source {
        ActivitySource::Cockpit => 'c',
        ActivitySource::WorkEvent => 'e',
    };
    if sequence < 0 {
        // %012d with a negative value: the '-' occupies one width slot.
        format!("{}-{:011}", prefix, sequence.unsigned_abs())
    } else {
        format!("{}{:012}", prefix, sequence)
    }
}

/// One feed row, exactly as the Go producer emits it. The caller supplies
/// **complete** rows: `origin`, `label` and `message` must already be
/// resolved by the producer; this module never re-derives them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ActivityEntry {
    /// Timestamp as written by the producer; opaque here, only compared.
    pub at: String,
    /// Write rank inside the source table, prefixed by source. Two rows may
    /// share `at`; without this discriminant the cursor would silently skip
    /// or duplicate them.
    pub rank: String,
    /// « moteur » or « vous », as resolved by the producer. Not inferred here.
    pub origin: String,
    /// Event kind as written; not classified here.
    pub kind: String,
    /// Human-readable label resolved by the producer; never rebuilt here.
    pub label: String,
    /// Task reference when the producer recorded one; omitted from JSON when
    /// empty (Go `task_id,omitempty`).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub task_id: String,
    /// Human-readable message resolved by the producer; never rebuilt here.
    pub message: String,
}

impl ActivityEntry {
    /// Pagination key: `at + "|" + rank`, compared as one byte string — the
    /// exact Go protocol. The separator matters: tuple-style comparison of
    /// `(at, rank)` is *not* equivalent when one `at` is a prefix of
    /// another, so the concatenation is what gets compared, as in Go.
    ///
    /// Pure: builds a new string, touches nothing else. Not a secret, not an
    /// authorization token; carrying it grants nothing.
    pub fn cursor(&self) -> String {
        let mut out = String::with_capacity(self.at.len() + 1 + self.rank.len());
        out.push_str(&self.at);
        out.push('|');
        out.push_str(&self.rank);
        out
    }
}

/// Page request, mirroring the Go `activityQuery`. Every policy value is
/// supplied by the caller; this module applies no default and no clamping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityQuery {
    /// Exclusive upper bound on cursor: rows with `cursor >= before` are
    /// excluded. Empty string means unbounded (start of the feed).
    pub before: String,
    /// Maximum rows to return. Must be at least 1 and should be the
    /// operator-persisted page size held by the caller; zero is refused,
    /// never defaulted, and no upper clamp is applied here.
    pub limit: usize,
    /// Keep only rows whose `origin` is exactly `ACTIVITY_ORIGIN_HUMAN`
    /// (« vous »). No kind-based inference is performed.
    pub decisions_only: bool,
}

/// Page result, JSON-shaped like the Go `ActivityPage`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ActivityPage {
    /// Selected rows, newest first. Serializes as `[]` when empty (never
    /// `null`), matching the Go constructor's non-nil slice.
    pub entries: Vec<ActivityEntry>,
    /// Cursor to pass as `before` for the next page. Empty unless `more` is
    /// true: emitting a cursor without a following page would send callers
    /// to an empty page, so it is withheld exactly as Go withholds it.
    pub next_cursor: String,
    /// True only when an *eligible supplied* row (post-filter) exists beyond the
    /// page. A page filled exactly to `limit` with nothing eligible after
    /// it is `more = false`.
    pub more: bool,
    /// Verbatim pass-through of the caller-established flag: a producer
    /// source hit its read ceiling, so the feed must say the history is
    /// shallower than it looks instead of implying "nothing older exists".
    /// This module never sets or clears it on its own and applies no read
    /// cap of its own.
    pub history_truncated: bool,
}

/// Refusal reasons. No error here can originate from I/O — there is none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityPageError {
    /// `limit == 0`. Refused rather than silently replaced by a default
    /// page size: inventing one would override the operator's persisted
    /// setting with a hard-coded value this module does not own.
    ZeroLimit,
}

impl std::fmt::Display for ActivityPageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityPageError::ZeroLimit => f.write_str(
                "activity query limit must be at least 1: zero is refused, not defaulted",
            ),
        }
    }
}

impl std::error::Error for ActivityPageError {}

/// Paginates complete producer rows exactly as the Go feed does.
///
/// # Contract
/// - **Pure**: no I/O, no clock, no globals, no logging; consumes `entries`
///   by value and writes nothing outside the returned page. A real caller
///   re-supplies its observed source window on each page request with an advanced
///   `before`; no pagination state is stored here. A finite source policy can
///   exclude older rows from every page. The truncation flag reports that
///   exclusion; increasing/removing the source cap is a separate operator choice.
/// - **Ordering**: rows are sorted by [`ActivityEntry::cursor`]
///   (`at + "|" + rank`) in descending byte-lexical order — identical to
///   Go's string comparison. Rust's sort is stable where Go's is not; for
///   distinct cursors (the only case a correct producer emits) the results
///   are identical, and duplicated cursors keep input order rather than an
///   unspecified one.
/// - **Filtering**: `decisions_only` keeps rows whose `origin` is exactly
///   `ACTIVITY_ORIGIN_HUMAN`; `before` excludes rows with
///   `cursor >= before` (exclusive, as in Go); empty `before` is unbounded.
/// - **Page end**: `more` becomes true only when an eligible row is found
///   beyond the `limit`-th selected row; `next_cursor` is the last selected
///   row's cursor and is emitted only when `more` is true.
/// - **Truncation**: `history_truncated` is passed through verbatim. This
///   function applies no per-source read cap and never trims rows on its
///   own; only the reader that touched the source can know it was
///   truncated.
/// - **Policy**: `limit` is trusted as supplied. Zero is refused; no
///   default is invented and no clamp to any hard-coded maximum is applied
///   — page-size policy belongs to the operator's persisted settings held
///   by the caller.
///
/// # Errors
/// [`ActivityPageError::ZeroLimit`] when `query.limit == 0`.
pub fn paginate_activity(
    entries: Vec<ActivityEntry>,
    query: &ActivityQuery,
    history_truncated: bool,
) -> Result<ActivityPage, ActivityPageError> {
    if query.limit == 0 {
        return Err(ActivityPageError::ZeroLimit);
    }

    // Cache each cursor once; both ordering and the `before` bound use the
    // concatenated byte string, exactly as Go compares it.
    let mut ranked: Vec<(String, ActivityEntry)> = entries
        .into_iter()
        .map(|entry| {
            let cursor = entry.cursor();
            (cursor, entry)
        })
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));

    // No with_capacity(query.limit): the limit is caller-supplied and must
    // not drive a blind allocation.
    let mut selected: Vec<ActivityEntry> = Vec::new();
    let mut more = false;
    for (cursor, entry) in ranked {
        if query.decisions_only && entry.origin != ACTIVITY_ORIGIN_HUMAN {
            continue;
        }
        if !query.before.is_empty() && cursor.as_str() >= query.before.as_str() {
            continue;
        }
        if selected.len() == query.limit {
            // An eligible row exists past the page: only now is there a next.
            more = true;
            break;
        }
        selected.push(entry);
    }

    let next_cursor = if more {
        // `more` implies a full page and `limit >= 1`; the guard mirrors the
        // Go condition anyway.
        selected
            .last()
            .map(ActivityEntry::cursor)
            .unwrap_or_default()
    } else {
        String::new()
    };

    Ok(ActivityPage {
        entries: selected,
        next_cursor,
        more,
        history_truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test fixture builder: producer-resolved rows, as the caller contract
    /// requires. Pure; no I/O.
    #[allow(clippy::too_many_arguments)]
    fn entry(
        at: &str,
        source: ActivitySource,
        sequence: i64,
        origin: &str,
        kind: &str,
        label: &str,
        task_id: &str,
        message: &str,
    ) -> ActivityEntry {
        ActivityEntry {
            at: at.to_string(),
            rank: activity_rank(source, sequence),
            origin: origin.to_string(),
            kind: kind.to_string(),
            label: label.to_string(),
            task_id: task_id.to_string(),
            message: message.to_string(),
        }
    }

    fn q(before: &str, limit: usize, decisions_only: bool) -> ActivityQuery {
        ActivityQuery {
            before: before.to_string(),
            limit,
            decisions_only,
        }
    }

    /// Two rows share a timestamp; only the rank tiebreak orders them, and
    /// the third row must surface on page two — never skipped in silence.
    #[test]
    fn same_timestamp_rank_pagination_does_not_skip_an_entry() {
        let at = "2024-05-01T10:00:00Z";
        let cockpit = entry(
            at,
            ActivitySource::Cockpit,
            7,
            "moteur",
            "dispatch",
            "Ordonnancement",
            "",
            "cycle lancé",
        );
        let work = entry(
            at,
            ActivitySource::WorkEvent,
            4,
            "vous",
            "decision",
            "Décision",
            "task-9",
            "priorité haute",
        );
        let older = entry(
            "2024-04-30T08:00:00Z",
            ActivitySource::Cockpit,
            2,
            "moteur",
            "conductor",
            "Conduite",
            "",
            "départs",
        );
        // Scrambled on purpose: ordering belongs to the function.
        let all = vec![older.clone(), work.clone(), cockpit.clone()];

        let page_one =
            paginate_activity(all.clone(), &q("", 2, false), false).expect("non-zero limit");
        // Same `at`: byte order puts the `e` rank above the `c` rank.
        assert_eq!(page_one.entries, vec![work.clone(), cockpit.clone()]);
        assert!(page_one.more, "a third eligible row exists beyond the page");
        let bound = page_one.next_cursor.clone();
        assert_eq!(bound, cockpit.cursor());

        let page_two = paginate_activity(all, &q(&bound, 2, false), false).expect("non-zero limit");
        // The shared timestamp did not swallow the third row.
        assert_eq!(page_two.entries, vec![older.clone()]);
        assert!(!page_two.more);
        assert_eq!(page_two.next_cursor, "");
        assert_eq!(
            page_one.entries.len() + page_two.entries.len(),
            3,
            "no entry skipped across pages"
        );
    }

    /// `before` is an exclusive bound and `decisions_only` keeps exactly
    /// « vous »: the engine row inside the window must not leak in.
    #[test]
    fn before_bound_and_decisions_only_filter_apply_together() {
        let newest_engine = entry(
            "2024-05-02T00:00:00Z",
            ActivitySource::WorkEvent,
            100,
            "moteur",
            "dispatch",
            "Ordonnancement",
            "",
            "cycle",
        );
        let human_decision = entry(
            "2024-05-01T12:00:00Z",
            ActivitySource::WorkEvent,
            90,
            "vous",
            "decision",
            "Décision",
            "",
            "priorité",
        );
        let engine_between = entry(
            "2024-05-01T11:00:00Z",
            ActivitySource::WorkEvent,
            80,
            "moteur",
            "conductor",
            "Conduite",
            "",
            "relais",
        );
        let human_pause = entry(
            "2024-05-01T10:00:00Z",
            ActivitySource::WorkEvent,
            70,
            "vous",
            "pause",
            "Départs",
            "",
            "pause demandée",
        );
        let human_gate = entry(
            "2024-05-01T09:00:00Z",
            ActivitySource::Cockpit,
            60,
            "vous",
            "gate",
            "Gate",
            "task-2",
            "évaluation",
        );
        let all = vec![
            human_pause.clone(),
            newest_engine,
            human_gate.clone(),
            human_decision.clone(),
            engine_between,
        ];
        let bound = human_decision.cursor();

        let page = paginate_activity(all, &q(&bound, 10, true), false).expect("non-zero limit");
        assert_eq!(page.entries, vec![human_pause, human_gate]);
        assert!(!page.more);
        assert_eq!(page.next_cursor, "");
        assert!(!page.history_truncated);
    }

    /// An empty selection must not erase the producer's truncation report:
    /// an empty page is not proof that the history ends here.
    #[test]
    fn no_eligible_row_keeps_source_truncation_visible() {
        let all = vec![
            entry(
                "2024-05-01T10:00:00Z",
                ActivitySource::Cockpit,
                1,
                "moteur",
                "dispatch",
                "Ordonnancement",
                "",
                "cycle",
            ),
            entry(
                "2024-05-01T09:00:00Z",
                ActivitySource::WorkEvent,
                1,
                "moteur",
                "conductor",
                "Conduite",
                "",
                "relais",
            ),
        ];
        let page = paginate_activity(all, &q("", 5, true), true).expect("non-zero limit");
        assert!(page.entries.is_empty());
        assert!(
            page.history_truncated,
            "caller-established truncation must survive an empty page"
        );
        assert!(!page.more);
        assert_eq!(page.next_cursor, "");
    }

    /// A page filled exactly to the limit has no next unless an eligible
    /// row actually lies beyond it.
    #[test]
    fn exact_page_without_following_row_has_no_spurious_next() {
        let newest = entry(
            "2024-05-01T10:00:00Z",
            ActivitySource::Cockpit,
            3,
            "vous",
            "decision",
            "Décision",
            "",
            "a",
        );
        let middle = entry(
            "2024-05-01T09:00:00Z",
            ActivitySource::Cockpit,
            2,
            "vous",
            "decision",
            "Décision",
            "",
            "b",
        );
        let oldest = entry(
            "2024-05-01T08:00:00Z",
            ActivitySource::Cockpit,
            1,
            "vous",
            "decision",
            "Décision",
            "",
            "c",
        );
        let all = vec![oldest.clone(), newest.clone(), middle.clone()];

        let exact =
            paginate_activity(all.clone(), &q("", 3, false), false).expect("non-zero limit");
        assert_eq!(
            exact.entries,
            vec![newest.clone(), middle.clone(), oldest.clone()]
        );
        assert!(
            !exact.more,
            "a page filled exactly to the limit is not 'more'"
        );
        assert_eq!(exact.next_cursor, "");

        let short = paginate_activity(all, &q("", 2, false), false).expect("non-zero limit");
        assert_eq!(short.entries, vec![newest, middle.clone()]);
        assert!(short.more);
        assert_eq!(short.next_cursor, middle.cursor());
    }

    /// `limit == 0` is refused outright: no invented default page size, no
    /// clamping fallback — the operator's setting belongs to the caller.
    #[test]
    fn zero_limit_is_refused_not_defaulted() {
        let rows = vec![entry(
            "2024-05-01T10:00:00Z",
            ActivitySource::Cockpit,
            1,
            "vous",
            "decision",
            "Décision",
            "",
            "a",
        )];
        let error = paginate_activity(rows, &q("", 0, false), false)
            .expect_err("zero limit is refused rather than replaced by a default page size");
        assert_eq!(error, ActivityPageError::ZeroLimit);
    }
}
