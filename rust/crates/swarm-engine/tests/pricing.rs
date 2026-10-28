use swarm_engine::{
    decode_rate_change, decode_rate_quote, quote_model_rate, validate_model_rate, GoFloat,
    ModelRate,
};

/// Synthetic prices test explicit category semantics, unknown versus free and
/// representational refusal. No provider tariff, IO or launch is exercised.
#[test]
fn pricing_explicit_categories_unknown_free_and_numeric_limits() {
    let change = decode_rate_change(br#"{"schema_version":1,"expected_digest":"fixture","rate":{"provider":"fixture","model":"model-v1","currency":"USD","input_per_million":2,"output_per_million":5,"source":"synthetic fixture","reference_date":"2026-10-19"}}"#).unwrap();
    validate_model_rate(&change.rate).unwrap();
    let mut rate = change.rate;
    rate.version = "rate-fixture".into();
    let mut volumes=decode_rate_quote(br#"{"rate_version":"rate-fixture","non_cached_input_tokens":1000000,"output_tokens":1000000,"cache_read_tokens":1000000}"#).unwrap();
    let unknown = quote_model_rate(&rate, &volumes).unwrap();
    assert_eq!(unknown.status, "unknown");
    assert_eq!(unknown.estimated_amount, None);
    assert_eq!(unknown.missing_rates, vec!["cache_read"]);
    rate.cache_read_per_million = Some(GoFloat(0.0));
    let known = quote_model_rate(&rate, &volumes).unwrap();
    assert_eq!(known.estimated_amount, Some(GoFloat(7.0)));
    rate.input_per_million = Some(GoFloat(1e10));
    validate_model_rate(&rate).unwrap();
    volumes.non_cached_input_tokens = Some(1000000000001);
    assert!(quote_model_rate(&rate, &volumes)
        .unwrap()
        .estimated_amount
        .unwrap()
        .0
        .is_finite());
    rate.input_per_million = Some(GoFloat(f64::MAX));
    volumes.non_cached_input_tokens = Some(i64::MAX);
    assert!(quote_model_rate(&rate, &volumes).is_err());
    volumes.non_cached_input_tokens = None;
    assert!(quote_model_rate(&rate, &volumes).is_err());
}

/// Untrusted malformed documents preserve codec causes and safe diagnostics;
/// date/model/source/finite constraints reject invalid proposals before IO.
#[test]
fn pricing_strict_codec_and_validation_preserve_safe_causes() {
    use std::error::Error;
    let err = decode_rate_change(br#"{"PRIVATE_unknown":1}"#).unwrap_err();
    assert!(err.source().is_some());
    assert!(!format!("{err:?}").contains("PRIVATE"));
    for (model, date, source, amount) in [
        ("bad model", "2026-10-19", "fixture", 1.0),
        ("valid", "2026-02-30", "fixture", 1.0),
        ("valid", "2026-10-19", "\u{a0}", 1.0),
        ("valid", "2026-10-19", "fixture", -1.0),
        ("valid", "2026-10-19", "fixture", f64::NAN),
    ] {
        let rate = ModelRate {
            provider: "fixture".into(),
            model: model.into(),
            currency: "USD".into(),
            reference_date: date.into(),
            source: source.into(),
            input_per_million: Some(GoFloat(amount)),
            ..Default::default()
        };
        assert!(validate_model_rate(&rate).is_err());
    }
}

#[cfg(unix)]
mod storage {
    use super::*;
    use std::{
        error::Error,
        fs,
        os::unix::fs::{symlink, PermissionsExt},
        path::PathBuf,
        sync::{Arc, Barrier},
    };
    use swarm_engine::{
        GitObservation, RootStore, RuntimeError, RuntimeOperation, SystemEnvironment,
        WorkEnvironment,
    };
    struct Fixture(PathBuf);
    impl Fixture {
        /// Private owned native root, independent of all Go/live roots; retained on panic.
        fn new() -> Self {
            let p = std::env::temp_dir()
                .join(swarm_engine::generate_identifier("pricing-root-").unwrap());
            fs::create_dir(&p).unwrap();
            fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
            RootStore::initialize(&p, &mut SystemEnvironment)
                .unwrap()
                .close()
                .unwrap();
            Self(p)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if !std::thread::panicking() {
                fs::remove_dir_all(&self.0).unwrap();
            }
        }
    }
    /// Synthetic complete intent; engine must replace these forged observation labels.
    fn change(digest: String) -> swarm_engine::RateChange {
        let mut c=decode_rate_change(br#"{"schema_version":1,"rate":{"provider":"fixture","model":"model-v1","currency":"USD","input_per_million":2,"output_per_million":5,"source":"synthetic fixture","reference_date":"2026-10-19","actor":"FORGED","updated":"FORGED","version":"FORGED"}}"#).unwrap();
        c.expected_digest = digest;
        c
    }
    /// Persist, reopen and quote exact old versions; stale input cannot append or
    /// rewrite history. A data-only 1,000-row fixture crosses the former count cap.
    #[test]
    fn pricing_root_persists_versions_and_preserves_history_without_count_policy() {
        let f = Fixture::new();
        let store = RootStore::open(&f.0, true).unwrap();
        let empty = store.rate_catalogue().unwrap();
        assert_eq!(empty.rates.as_ref().unwrap().len(), 0);
        assert!(!f.0.join(".swarm/model-rates.json").exists());
        let request = change(empty.digest);
        let first = store.save_rate(&request, &mut SystemEnvironment).unwrap();
        assert_ne!(first.actor, "FORGED");
        assert_ne!(first.updated, "FORGED");
        assert_ne!(first.version, "FORGED");
        let path = f.0.join(".swarm/model-rates.json");
        let original = fs::read(&path).unwrap();
        assert_eq!(
            store
                .save_rate(&request, &mut SystemEnvironment)
                .unwrap_err()
                .code(),
            "revision_conflict"
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        let c = store.rate_catalogue().unwrap();
        let mut next = change(c.digest);
        next.rate.input_per_million = Some(GoFloat(3.0));
        let second = store.save_rate(&next, &mut SystemEnvironment).unwrap();
        assert_ne!(first.version, second.version);
        store.close().unwrap();
        let store = RootStore::open(&f.0, false).unwrap();
        let c = store.rate_catalogue().unwrap();
        let quote=decode_rate_quote(format!("{{\"rate_version\":\"{}\",\"non_cached_input_tokens\":1000000,\"output_tokens\":0}}",first.version).as_bytes()).unwrap();
        assert_eq!(
            c.quote(&quote).unwrap().estimated_amount,
            Some(GoFloat(2.0))
        );
        assert_eq!(
            store
                .save_rate(&next, &mut SystemEnvironment)
                .unwrap_err()
                .code(),
            "root-read-only"
        );
        store.close().unwrap();
        let mut rows = vec![];
        for i in 0..1000 {
            let mut rate = first.clone();
            rate.version = format!("rate-fixture-{i}");
            rows.push(rate);
        }
        let catalogue = swarm_engine::RateCatalogue {
            schema_version: 1,
            digest: String::new(),
            rates: Some(rows),
        };
        fs::write(&path, swarm_engine::encode_model(&catalogue).unwrap()).unwrap();
        let store = RootStore::open(&f.0, true).unwrap();
        let c = store.rate_catalogue().unwrap();
        store
            .save_rate(&change(c.digest), &mut SystemEnvironment)
            .unwrap();
        assert_eq!(store.rate_catalogue().unwrap().rates.unwrap().len(), 1001);
        store.close().unwrap();
    }
    /// Refusals preserve original catalogue/target bytes, and repairing only private
    /// fixture data permits the same read/save intent again. No live state is edited.
    #[test]
    fn pricing_root_corruption_symlink_and_oversize_refuse_then_recover() {
        let f = Fixture::new();
        let store = RootStore::open(&f.0, true).unwrap();
        let c = store.rate_catalogue().unwrap();
        store
            .save_rate(&change(c.digest), &mut SystemEnvironment)
            .unwrap();
        let path = f.0.join(".swarm/model-rates.json");
        let original = fs::read(&path).unwrap();
        let mut c = store.rate_catalogue().unwrap();
        c.rates.as_mut().unwrap()[0].input_per_million = Some(GoFloat(-1.0));
        let bad = swarm_engine::encode_model(&c).unwrap();
        fs::write(&path, &bad).unwrap();
        assert!(store.rate_catalogue().is_err());
        assert!(store
            .save_rate(&change(c.digest), &mut SystemEnvironment)
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), bad);
        fs::write(&path, &original).unwrap();
        assert!(store.rate_catalogue().is_ok());
        fs::write(
            &path,
            vec![b' '; swarm_engine::RATE_CATALOGUE_MAX_BYTES + 1],
        )
        .unwrap();
        assert!(store.rate_catalogue().is_err());
        assert_eq!(
            fs::metadata(&path).unwrap().len(),
            (swarm_engine::RATE_CATALOGUE_MAX_BYTES + 1) as u64
        );
        fs::remove_file(&path).unwrap();
        let outside = f.0.join("unchanged-target");
        fs::write(&outside, &original).unwrap();
        symlink(&outside, &path).unwrap();
        assert!(store.rate_catalogue().is_err());
        assert_eq!(fs::read(&outside).unwrap(), original);
        fs::remove_file(&path).unwrap();
        fs::write(&path, &original).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(store.rate_catalogue().is_ok());
        store.close().unwrap();
    }
    struct InvalidClock;
    impl WorkEnvironment for InvalidClock {
        fn identifier(&mut self, _: &str) -> Result<String, RuntimeError> {
            Ok("rate-fixed".into())
        }
        fn operator(&mut self) -> Result<String, RuntimeError> {
            Ok("fixture".into())
        }
        fn timestamp(&mut self) -> Result<String, RuntimeError> {
            Ok("PRIVATE invalid clock".into())
        }
        fn git(&mut self, _: &std::path::Path) -> Result<GitObservation, RuntimeError> {
            Err(RuntimeError::new(
                RuntimeOperation::Platform,
                std::io::Error::other("git must not be called"),
            ))
        }
    }
    /// Invalid engine observations preserve a typed cause and create no catalogue;
    /// concurrent native roots sharing a digest produce one append and one refusal.
    #[test]
    fn pricing_root_clock_failure_recovers_and_concurrent_writes_are_serialized() {
        let f = Fixture::new();
        let a = RootStore::open(&f.0, true).unwrap();
        let request = change(a.rate_catalogue().unwrap().digest);
        let error = a.save_rate(&request, &mut InvalidClock).unwrap_err();
        assert_eq!(error.code(), "pricing-engine-clock-invalid");
        assert!(error.source().unwrap().source().is_some());
        assert!(!format!("{error:?}").contains("PRIVATE"));
        assert!(!f.0.join(".swarm/model-rates.json").exists());
        let b = RootStore::open(&f.0, true).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let mut jobs = vec![];
        for store in [a, b] {
            let request = request.clone();
            let barrier = barrier.clone();
            jobs.push(std::thread::spawn(move || {
                barrier.wait();
                let result = store.save_rate(&request, &mut SystemEnvironment);
                store.close().unwrap();
                result
            }));
        }
        let results: Vec<_> = jobs.into_iter().map(|job| job.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| r.as_ref().is_err_and(|e| e.code() == "revision_conflict"))
                .count(),
            1
        );
        let store = RootStore::open(&f.0, false).unwrap();
        assert_eq!(store.rate_catalogue().unwrap().rates.unwrap().len(), 1);
        store.close().unwrap();
    }
    /// Inject storage failures through real publication after a valid witness.
    /// Pre-rename failures preserve exact original bytes; after rename, explicit
    /// durability uncertainty requires read-back and the old digest cannot append.
    #[cfg(feature = "test-fixtures")]
    #[test]
    fn pricing_root_storage_failures_preserve_history_and_expose_uncertain_commit() {
        use swarm_engine::{AtomicStorageFault, LocalStorageError, PublicationState};
        let f = Fixture::new();
        let store = RootStore::open(&f.0, true).unwrap();
        let c = store.rate_catalogue().unwrap();
        store
            .save_rate(&change(c.digest), &mut SystemEnvironment)
            .unwrap();
        let path = f.0.join(".swarm/model-rates.json");
        for fault in [
            AtomicStorageFault::PartialWriteNoSpace,
            AtomicStorageFault::SyncFile,
            AtomicStorageFault::CloseFile,
            AtomicStorageFault::Rename,
        ] {
            let before = fs::read(&path).unwrap();
            let request = change(store.rate_catalogue().unwrap().digest);
            let error = store
                .fixture_save_rate(&request, &mut SystemEnvironment, fault)
                .unwrap_err();
            let storage = error
                .source()
                .unwrap()
                .downcast_ref::<LocalStorageError>()
                .unwrap();
            assert_eq!(storage.publication, PublicationState::NotPublished);
            assert_eq!(fs::read(&path).unwrap(), before);
            store.save_rate(&request, &mut SystemEnvironment).unwrap();
        }
        let count = store.rate_catalogue().unwrap().rates.unwrap().len();
        let request = change(store.rate_catalogue().unwrap().digest);
        let error = store
            .fixture_save_rate(
                &request,
                &mut SystemEnvironment,
                AtomicStorageFault::SyncDirectory,
            )
            .unwrap_err();
        assert_eq!(error.code(), "local-publication-uncertain");
        let storage = error
            .source()
            .unwrap()
            .downcast_ref::<LocalStorageError>()
            .unwrap();
        assert_eq!(
            storage.publication,
            PublicationState::RenamedDurabilityUnknown
        );
        assert_eq!(
            store.rate_catalogue().unwrap().rates.unwrap().len(),
            count + 1
        );
        assert_eq!(
            store
                .save_rate(&request, &mut SystemEnvironment)
                .unwrap_err()
                .code(),
            "revision_conflict"
        );
        store.close().unwrap();
    }
}

/// Go rateCatalogue starts schema1 and an empty allocated slice before unmarshal.
/// Missing/scalar-null fields retain those initial values; explicit rates:null
/// clears the slice. All variants remain empty catalogues, never default prices.
#[test]
fn pricing_catalogue_preserves_go_initialized_defaults_and_explicit_null() {
    for raw in [b"{}".as_slice(), b"null", br#"{"schema_version":null}"#] {
        let c = swarm_engine::RateCatalogue::decode(raw).unwrap();
        assert_eq!(c.schema_version, 1);
        assert_eq!(c.rates.unwrap().len(), 0);
    }
    let c =
        swarm_engine::RateCatalogue::decode(br#"{"schema_version":null,"rates":null}"#).unwrap();
    assert_eq!(c.schema_version, 1);
    assert!(c.rates.is_none());
}

/// Actual legacy Go 3-token synthetic0.01 quote preserves its finite evaluation
/// order; rescaling is reserved for a product that would otherwise overflow.
#[test]
fn pricing_quote_keeps_legacy_finite_arithmetic_and_rescales_only_overflow() {
    let change=decode_rate_change(br#"{"schema_version":1,"rate":{"provider":"fixture","model":"v1","currency":"USD","input_per_million":0.01,"source":"synthetic","reference_date":"2026-10-19"}}"#).unwrap();
    let mut rate = change.rate;
    rate.version = "rate-fixture".into();
    let mut request = decode_rate_quote(
        br#"{"rate_version":"rate-fixture","non_cached_input_tokens":3,"output_tokens":0}"#,
    )
    .unwrap();
    let amount = quote_model_rate(&rate, &request)
        .unwrap()
        .estimated_amount
        .unwrap()
        .0;
    assert_eq!(amount.to_bits(), 3e-8_f64.to_bits());
    rate.input_per_million = Some(GoFloat(f64::MAX));
    request.non_cached_input_tokens = Some(1000000);
    assert_eq!(
        quote_model_rate(&rate, &request).unwrap().estimated_amount,
        Some(GoFloat(f64::MAX))
    );
}
