//! Audit aggregates and purge: sessions grouping, per-day/account dashboard,
//! purge keeping the chain verifiable and the tail unchanged, repeated purge,
//! refusal on a broken chain, UTC day computation.
use swarm_authority::audit::{dashboard, sessions, utc_day, Audit, Event};

fn ev<'a>(at: i64, event: &'a str, subject: &'a str, cred: &'a str) -> Event<'a> {
    Event { at, event, subject: Some(subject), credential: Some(cred), ..Default::default() }
}

#[test]
fn aggregates_purge_and_chain() {
    assert_eq!(utc_day(0), "1970-01-01");
    assert_eq!(utc_day(1_793_577_600), "2026-11-02");
    let area = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    let dir = tempfile::tempdir_in(area).unwrap();
    let a = Audit::open(dir.path()).unwrap();
    let day = 86_400;
    for e in [
        ev(10, "sign-in", "ana", "session:aa"),
        ev(11, "consultation", "ana", "session:aa"),
        ev(12, "intent", "ana", "session:aa"),
        ev(13, "outcome", "ana", "session:aa"),
        ev(14, "refused", "ana", "session:aa"),
        Event { at: 15, event: "sign-in-refused", subject: Some("eve"), ..Default::default() },
        ev(day + 5, "sign-in", "bob", "session:bb"),
        ev(day + 6, "consultation", "bob", "session:bb"),
        ev(day + 7, "sign-out", "bob", "session:bb"),
        ev(2 * day + 1, "intent", "ana", "key:0123456789abcdef"),
    ] {
        a.record(&e).unwrap();
    }
    let (entries, valid) = a.read().unwrap();
    assert!(valid && entries.len() == 10);
    let s = sessions(&entries, 0, i64::MAX, None);
    assert_eq!(s.iter().map(|x| x.credential.as_str()).collect::<Vec<_>>(), ["key:0123456789abcdef", "session:bb", "session:aa"]);
    let aa = s.iter().find(|x| x.credential == "session:aa").unwrap();
    assert_eq!((aa.consultations, aa.actions, aa.refusals, aa.signed_out), (1, 1, 1, false));
    assert!(s.iter().find(|x| x.credential == "session:bb").unwrap().signed_out);
    assert_eq!(sessions(&entries, 0, i64::MAX, Some("bob")).len(), 1);
    assert_eq!(sessions(&entries, day, 2 * day, None).len(), 1, "period filter");
    let (days, accounts) = dashboard(&entries, 0, i64::MAX);
    assert_eq!(days.len(), 3);
    assert_eq!((days[0].sign_ins, days[0].refused_sign_ins, days[0].accounts, days[0].actions), (1, 1, 1, 1));
    assert_eq!((accounts[0].subject.as_str(), accounts[0].actions, accounts[0].sessions), ("ana", 2, 2));
    assert!(accounts.iter().all(|a| a.subject != "eve"), "refused sign-ins are not account activity");

    // Purge removes the oldest prefix (lines are appended in time order),
    // keeps the chain verifiable and the tail (witness) unchanged.
    let tail = a.tail().unwrap();
    assert_eq!(a.purge(day, "ana", 100), Ok(6), "lines before day 1 removed");
    let (after, valid) = a.read().unwrap();
    assert!(valid, "chain verifiable from the anchor");
    assert_eq!(after[0].event, "purge-anchor");
    assert_eq!((after[0].subject.as_deref(), after[0].action.as_deref()), (Some("ana"), Some("removed:6")));
    assert_eq!(after.len(), 5);
    assert_eq!(a.tail().unwrap(), tail);
    a.record(&ev(3 * day, "consultation", "ana", "session:cc")).unwrap();
    assert!(a.read().unwrap().1, "appending after a purge keeps the chain");
    // Nothing older: no rewrite. A second purge replaces the anchor.
    assert_eq!(a.purge(1, "ana", 101), Ok(0));
    assert_eq!(a.purge(2 * day, "ana", 102), Ok(3));
    let (again, valid) = a.read().unwrap();
    assert!(valid && again.iter().filter(|e| e.event == "purge-anchor").count() == 1);
    let reopened = Audit::open(dir.path()).unwrap();
    assert_eq!(reopened.tail().unwrap(), a.tail().unwrap(), "reopen continues the same chain");
    // A broken chain refuses the purge (it would hide the break).
    let file = dir.path().join("audit.jsonl");
    let text = std::fs::read_to_string(&file).unwrap().replacen("\"intent\"", "\"outcome\"", 1);
    std::fs::write(&file, text).unwrap();
    assert_eq!(Audit::open(dir.path()).unwrap().purge(i64::MAX, "ana", 103), Err("authority-audit-chain-broken"));
}
