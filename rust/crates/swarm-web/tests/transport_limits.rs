//! Pin/probe technical limits separately from operator defaults; no network traffic.
use swarm_engine::{EngineSettingsValues, WebTransportConstraints};
/// http1.5 HeaderMap usable capacity is24576, not its raw32768slot backing table.
/// The next capacity must fail so a dependency update cannot silently stale metadata.
#[test]
fn header_map_capacity_matches_declared_representation_bound() {
    let l = WebTransportConstraints::current();
    let n = usize::try_from(l.max_header_fields.0).unwrap();
    assert!(http::HeaderMap::<()>::try_with_capacity(n).is_ok());
    assert!(http::HeaderMap::<()>::try_with_capacity(n + 1).is_err());
}
/// Explicit policy can request a tiny header cap while the library's permitted
/// buffer ceiling stays at least8192. This does not claim allocated bytes or TCP.
#[test]
fn hyper_minimum_buffer_ceiling_is_distinct_from_operator_header_limit() {
    let l = WebTransportConstraints::current();
    let minimum = usize::try_from(l.minimum_connection_buffer_bytes.0).unwrap();
    let mut v = EngineSettingsValues::installation_defaults().unwrap();
    v.web_http_header_max_bytes = Some(swarm_engine::SettingCount(1));
    let p = v.web_transport_policy().unwrap();
    assert_eq!(p.header_max_bytes, 1);
    let mut builder = hyper::server::conn::http1::Builder::new();
    builder
        .max_buf_size(minimum.max(p.header_max_bytes))
        .max_header_size(p.header_max_bytes)
        .max_headers(p.max_headers)
        .header_read_timeout(p.header_timeout);
    assert!(std::panic::catch_unwind(|| {
        hyper::server::conn::http1::Builder::new().max_buf_size(minimum - 1);
    })
    .is_err());
}
/// Tokio's actual exported permit limit must agree with engine validation. This
/// constant protects representation, not hardware/resource availability or runtime adoption.
#[test]
fn tokio_permit_representation_matches_declared_constraint() {
    assert_eq!(
        usize::try_from(WebTransportConstraints::current().max_semaphore_permits.0).unwrap(),
        tokio::sync::Semaphore::MAX_PERMITS
    );
}
