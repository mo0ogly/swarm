use swarm_engine::review_evidence_present as present;

/// Literal evidence remains valid in the same artifact before every malformed-close refusal.
#[test]
fn matching_fences_keep_normalization_out_of_code_and_resume_prose() {
    for kind in ["source", "diff"] {
        for content in [
            "```go\n~~~\nreturn `raw` text\n```",
            "````go\n```\nreturn `raw` text\n````",
            "```go\n```not-a-close\nreturn `raw` text\n```",
        ] {
            let raw = if kind == "diff" {
                format!("+{}", content.replace('\n', "\n+"))
            } else {
                content.into()
            };
            assert!(present("report.md", kind, &raw, "return `raw` text", 8).unwrap());
            assert!(!present("report.md", kind, &raw, "return raw text", 8).unwrap());
            assert!(present("report.md", kind, &raw, "return `raw` text", 8).unwrap());
        }
        for (opening, closing) in [("```go", "```"), ("~~~~go", "~~~~~"), ("````go", "`````")] {
            let content =
                format!("{opening}\nreturn `raw` text\n{closing}\nValid `prose` after\nboundary.");
            let raw = if kind == "diff" {
                format!("+{}", content.replace('\n', "\n+"))
            } else {
                content
            };
            assert!(present("report.md", kind, &raw, "return `raw` text", 8).unwrap());
            assert!(!present("report.md", kind, &raw, "return raw text", 8).unwrap());
            assert!(present("report.md", kind, &raw, "Valid prose after boundary.", 8).unwrap());
        }
    }
}
/// Prose permissions and diff boundaries never turn code/context or separate runs into a quote.
#[test]
fn source_kind_path_whitespace_and_diff_runs_preserve_quote_boundaries() {
    let quote = "Original exact evidence";
    let raw = "Original `exact`\nevidence";
    assert!(present("proof.md", "source", raw, quote, 8).unwrap());
    for (name, kind) in [
        ("proof.go", "source"),
        ("proof.MD", "source"),
        ("proof.md", "context"),
        ("proof.md", "control_output"),
    ] {
        assert!(present(name, kind, raw, "`exact`", 8).unwrap());
        assert!(!present(name, kind, raw, quote, 8).unwrap());
    }
    for raw in [
        "+Original `exact`\n-evidence",
        "+Original `exact`\n@@ -1 +1 @@\n+evidence",
        "+Original `exact`\n+\n+evidence",
        "+Original `exact`\n+++ next.md\n+evidence",
    ] {
        assert!(!present("proof.md", "diff", raw, quote, 8).unwrap());
    }
    for space in [
        '\t', '\r', '\u{0085}', '\u{00a0}', '\u{1680}', '\u{200a}', '\u{2028}', '\u{202f}',
        '\u{205f}', '\u{3000}',
    ] {
        assert!(present(
            "proof.md",
            "source",
            &format!("Original{space}`exact`{space}evidence"),
            quote,
            8
        )
        .unwrap());
    }
    for space in ['\u{001c}', '\u{200b}', '\u{feff}'] {
        assert!(!present(
            "proof.md",
            "source",
            &format!("Original{space}`exact`{space}evidence"),
            quote,
            8
        )
        .unwrap());
    }
}
/// The explicit minimum affects only normalized fallback; this predicate is no complete reply validator.
#[test]
fn fallback_minimum_is_an_explicit_byte_argument_not_a_hidden_root_policy() {
    let raw = "Original `exact` evidence";
    let quote = "Original exact evidence";
    assert!(present("proof.md", "source", raw, quote, quote.len()).unwrap());
    assert!(!present("proof.md", "source", raw, quote, quote.len() + 1).unwrap());
    assert!(present("proof.go", "context", "literal", "literal", usize::MAX).unwrap());
    assert!(present("proof.go", "context", "literal", "", 8).unwrap());
    let raw = "éé `éé`";
    let quote = "éé éé";
    assert_eq!(quote.len(), 9);
    assert!(present("proof.md", "source", raw, quote, 9).unwrap());
    assert!(!present("proof.md", "source", raw, quote, 10).unwrap());
}
