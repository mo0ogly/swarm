//! HTTP status of engine refusals that are declared port boundaries.
use super::*;

#[test]
fn planning_status_port_boundary_is_not_implemented_not_a_server_error() {
    let unported = root("read", RootStoreError::Refused("mission-status-planning-unported"));
    assert_eq!(unported.status(), StatusCode::NOT_IMPLEMENTED);
    // Witness: other read refusals keep their previous classification.
    let other = root("read", RootStoreError::Refused("mission-root-unavailable"));
    assert_ne!(other.status(), StatusCode::NOT_IMPLEMENTED);
    // The same code under another operation is not reclassified.
    assert_ne!(root("mutate", RootStoreError::Refused("mission-status-planning-unported")).status(), StatusCode::NOT_IMPLEMENTED);
}
