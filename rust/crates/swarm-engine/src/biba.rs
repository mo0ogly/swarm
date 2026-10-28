//! Biba integrity guard (rust/crates/swarm-engine/src/biba.rs)
//!
//! Strict Biba model, project contract invariants (not operator policy):
//! - Read:    allowed iff subject <= object (no read down)
//! - Write:   allowed iff subject >= object (no write up)
//! - Invoke:  allowed iff subject >= object (no invoke up)
//!
//! This generic guard performs no authentication, no persistence, no
//! provenance checks, and provides no endorsement path. Callers owe:
//! authenticated subject level, persisted policy, object provenance,
//! current transactional authority, and (separately, outside this file)
//! any narrow named endorsement that produces a NEW object. No client
//! level or role authorizes itself here; there is no bypass in this guard.
//!
//! Limits: 4 fixed integrity levels (I0..I3), 3 access kinds; total
//! decision space is 4*4*3 = 48 cases, fully covered by tests below.

use std::error::Error;
use std::fmt;

/// Integrity levels. I0 is lowest, I3 is highest.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Integrity {
    I0,
    I1,
    I2,
    I3,
}

/// Kinds of Biba-governed access.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BibaAccess {
    Read,
    Write,
    Invoke,
}

/// Denial record: exact subject level, object level, and refused access.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BibaDenied {
    pub subject: Integrity,
    pub object: Integrity,
    pub access: BibaAccess,
}

impl fmt::Display for BibaDenied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Biba denied: subject {:?} cannot {:?} object {:?}",
            self.subject, self.access, self.object
        )
    }
}

impl Error for BibaDenied {}

/// Ordinal rank of an integrity level (0..=3). Private test-adjacent helper;
/// tests below do NOT rely on it (see "mathematical independence" note).
fn rank(l: Integrity) -> u8 {
    match l {
        Integrity::I0 => 0,
        Integrity::I1 => 1,
        Integrity::I2 => 2,
        Integrity::I3 => 3,
    }
}

/// Check a Biba access. Returns Ok(()) or a BibaDenied describing the refusal.
///
/// Invariants (contract):
///   Read   <=> rank(subject) <= rank(object)
///   Write  <=> rank(subject) >= rank(object)
///   Invoke <=> rank(subject) >= rank(object)
pub fn check_biba(
    subject: Integrity,
    object: Integrity,
    access: BibaAccess,
) -> Result<(), BibaDenied> {
    let s = rank(subject);
    let o = rank(object);
    let allowed = match access {
        BibaAccess::Read => s <= o,
        BibaAccess::Write | BibaAccess::Invoke => s >= o,
    };
    if allowed {
        Ok(())
    } else {
        Err(BibaDenied {
            subject,
            object,
            access,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
//
// Mathematical independence: the expected verdicts below are written as an
// explicit hardcoded 48-entry truth table derived directly from the contract
// invariants (a hand-computed matrix), NOT by calling check_biba, rank(), or
// any helper. No helper output is used to compute expectations, so the tests
// are not tautological with respect to the implementation.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use Integrity::{I0, I1, I2, I3};

    /// Literal contract matrices, independent from the implementation rank helper.
    fn expected(s: Integrity, o: Integrity, a: BibaAccess) -> bool {
        let levels = [I0, I1, I2, I3];
        let si = levels.iter().position(|v| *v == s).unwrap();
        let oi = levels.iter().position(|v| *v == o).unwrap();
        const READ: [[bool; 4]; 4] = [
            [true, true, true, true],
            [false, true, true, true],
            [false, false, true, true],
            [false, false, false, true],
        ];
        const WRITE: [[bool; 4]; 4] = [
            [true, false, false, false],
            [true, true, false, false],
            [true, true, true, false],
            [true, true, true, true],
        ];
        match a {
            BibaAccess::Read => READ[si][oi],
            BibaAccess::Write | BibaAccess::Invoke => WRITE[si][oi],
        }
    }

    #[test]
    fn exhaustive_4x4x3_matrix() {
        let levels = [I0, I1, I2, I3];
        let accesses = [BibaAccess::Read, BibaAccess::Write, BibaAccess::Invoke];
        for &s in &levels {
            for &o in &levels {
                for &a in &accesses {
                    let got = check_biba(s, o, a).is_ok();
                    assert_eq!(got, expected(s, o, a), "s={s:?} o={o:?} a={a:?}");
                }
            }
        }
    }

    #[test]
    fn read_equal_levels_allowed() {
        for l in [I0, I1, I2, I3] {
            assert!(check_biba(l, l, BibaAccess::Read).is_ok());
        }
    }

    #[test]
    fn write_invoke_equal_levels_allowed() {
        for l in [I0, I1, I2, I3] {
            assert!(check_biba(l, l, BibaAccess::Write).is_ok());
            assert!(check_biba(l, l, BibaAccess::Invoke).is_ok());
        }
    }

    #[test]
    fn read_down_denied_boundary() {
        assert!(check_biba(I1, I0, BibaAccess::Read).is_err());
        assert!(check_biba(I3, I2, BibaAccess::Read).is_err());
    }

    #[test]
    fn write_up_denied_boundary() {
        assert!(check_biba(I0, I1, BibaAccess::Write).is_err());
        assert!(check_biba(I2, I3, BibaAccess::Invoke).is_err());
    }

    #[test]
    fn extremes() {
        assert!(check_biba(I0, I3, BibaAccess::Read).is_ok());
        assert!(check_biba(I3, I0, BibaAccess::Read).is_err());
        assert!(check_biba(I3, I0, BibaAccess::Write).is_ok());
        assert!(check_biba(I0, I3, BibaAccess::Write).is_err());
        assert!(check_biba(I3, I0, BibaAccess::Invoke).is_ok());
        assert!(check_biba(I0, I3, BibaAccess::Invoke).is_err());
    }

    #[test]
    fn denial_payload_and_traits() {
        let d = BibaDenied {
            subject: I2,
            object: I1,
            access: BibaAccess::Read,
        };
        assert_eq!(
            d,
            BibaDenied {
                subject: I2,
                object: I1,
                access: BibaAccess::Read
            }
        );
        assert_eq!(d.subject, I2);
        assert_eq!(d.access, BibaAccess::Read);
        let copied = d; // Copy
        assert_eq!(copied, d);
        assert!(!format!("{d}").is_empty());
        assert!(!format!("{d:?}").is_empty());
        // std::error::Error reachable via a generic bound check
        fn takes_error<E: Error>(_: &E) -> bool {
            true
        }
        assert!(takes_error(&d));
    }

    #[test]
    fn err_payload_matches_inputs() {
        let r = check_biba(I1, I2, BibaAccess::Write).unwrap_err();
        assert_eq!(
            r,
            BibaDenied {
                subject: I1,
                object: I2,
                access: BibaAccess::Write
            }
        );
    }
}
