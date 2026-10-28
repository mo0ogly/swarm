//! Real disposable service-root publication; no OS account or human acceptance.
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::Path,
    sync::Arc,
};
use swarm_enrollment_authority::{
    self as a,
    provisioning::{self as p, service_artifact as artifact, service_installation as install},
};
use swarm_enrollment_config::store::documents::Publication;
use zeroize::Zeroizing;
/// Existing explicit synthetic request, rebound only to this isolated root/euid.
fn material(path: &Path) -> p::Prepared {
    let mut r: p::Request = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    r.root_directory = path.to_str().unwrap().into();
    r.service_uid = rustix::process::geteuid().as_raw();
    r.controller_uid = if r.service_uid == 0 {
        1
    } else {
        !r.service_uid
    };
    p::prepare(&r).unwrap()
}
/// Private parent with no live database; its child is deliberately initially absent.
fn fixture() -> (tempfile::TempDir, p::Prepared, artifact::ServiceArtifact) {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let p = material(&d.path().join("service"));
    let a = artifact::from_prepared(&p).unwrap();
    (d, p, a)
}
/// Exact sibling staging name is a versioned installer protocol, not fallback.
fn stage(a: &artifact::ServiceArtifact) -> std::path::PathBuf {
    let root = Path::new(a.root_directory());
    root.with_file_name(format!(
        "{}.paea-stage-v1",
        root.file_name().unwrap().to_str().unwrap()
    ))
}
/// Valid witness before each injected fault, with the same actual filesystem/API.
fn valid() {
    let (_d, _p, a) = fixture();
    let r = install::install(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::Create,
    )
    .unwrap();
    assert_eq!(r.progress.published, Publication::Durable);
}
/// Published roots feed the real protected lifecycle; no hand-built DB or account.
#[test]
fn service_installation_publishes_exact_root_and_actual_lifecycle() {
    let (_d, p, a) = fixture();
    let r = install::install(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::Create,
    )
    .unwrap();
    assert_eq!(r.progress.published, Publication::Durable);
    assert!(!stage(&a).exists());
    let ctx = a::authenticated_control::Context::new();
    let lifecycle = a::authenticated_startup::open_lifecycle(
        &swarm_enrollment_state::Context::new(),
        &r.bootstrap_path,
        &r.bootstrap_pin,
        Path::new(&r.personal_seed_path),
    )
    .unwrap();
    lifecycle.check_bootstrap(&ctx).unwrap();
    lifecycle.initialize(&ctx).unwrap();
    let mut proof = Zeroizing::new(Vec::with_capacity(32));
    p.write_installation_proof(&mut *proof).unwrap();
    let account = lifecycle
        .enroll(
            &ctx,
            proof.as_slice().try_into().unwrap(),
            b"synthetic-publisher-password",
        )
        .unwrap();
    assert_eq!(account.head().sequence, 3);
    assert_eq!(lifecycle.recover(&ctx).unwrap().head(), account.head());
    lifecycle.close().unwrap();
    assert_eq!(format!("{r:?}"), "ServiceInstallation { <private> }");
}
/// Pin/euid/existing final name must refuse before creating a stage.
#[test]
fn service_installation_wrong_pin_uid_and_existing_destination_refuse() {
    valid();
    let (_d, _p, a) = fixture();
    assert!(install::install(
        &install::Context::new(),
        &a,
        &"00".repeat(32),
        install::Mode::Create
    )
    .is_err());
    assert!(!stage(&a).exists());
    fs::create_dir(a.root_directory()).unwrap();
    fs::write(Path::new(a.root_directory()).join("original"), b"KEEP").unwrap();
    assert!(install::install(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::Create
    )
    .is_err());
    assert_eq!(
        fs::read(Path::new(a.root_directory()).join("original")).unwrap(),
        b"KEEP"
    );
    assert!(!stage(&a).exists());
    let (d, _p, _a) = fixture();
    let mut r: p::Request = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    r.root_directory = d.path().join("wrong-uid").to_str().unwrap().into();
    r.service_uid = !rustix::process::geteuid().as_raw();
    r.controller_uid = rustix::process::geteuid().as_raw();
    let bad = artifact::from_prepared(&p::prepare(&r).unwrap()).unwrap();
    assert!(install::install(
        &install::Context::new(),
        &bad,
        &bad.pin().unwrap(),
        install::Mode::Create
    )
    .is_err());
    assert!(!stage(&bad).exists());
}
/// Exact signed bytes replay after each private durable stage; final name absent.
#[test]
fn service_installation_interrupted_stages_resume_exactly() {
    for phase in [
        install::FixturePhase::IntentDurable,
        install::FixturePhase::BusinessApplied,
        install::FixturePhase::PersonalApplied,
        install::FixturePhase::ReadyToPublish,
    ] {
        valid();
        let (_d, _p, a) = fixture();
        let e = install::install_with_hook(
            &install::Context::new(),
            &a,
            &a.pin().unwrap(),
            install::Mode::Create,
            Arc::new(move |p| {
                if p == phase {
                    Err(install::Error::Interrupted)
                } else {
                    Ok(())
                }
            }),
        )
        .unwrap_err();
        assert_eq!(e.progress.published, Publication::NotAttempted);
        assert!(!Path::new(a.root_directory()).exists());
        let r = install::install(
            &install::Context::new(),
            &a,
            &a.pin().unwrap(),
            install::Mode::ResumeStage,
        )
        .unwrap();
        assert_eq!(r.progress.published, Publication::Durable);
    }
}
/// A real rename remains visible through error; explicit published recovery fsyncs.
#[test]
fn service_installation_visible_rename_error_retains_effect_and_recovery() {
    valid();
    let (_d, _p, a) = fixture();
    let e = install::install_with_hook(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::Create,
        Arc::new(|p| {
            if p == install::FixturePhase::RootRenamed {
                Err(install::Error::Interrupted)
            } else {
                Ok(())
            }
        }),
    )
    .unwrap_err();
    assert_eq!(e.progress.published, Publication::Visible);
    assert!(Path::new(a.root_directory()).is_dir());
    assert!(!stage(&a).exists());
    let before = fs::read(Path::new(a.root_directory()).join("business/journal.sqlite")).unwrap();
    let r = install::install(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::RecoverPublished,
    )
    .unwrap();
    assert_eq!(r.progress.published, Publication::Durable);
    assert_eq!(
        fs::read(Path::new(a.root_directory()).join("business/journal.sqlite")).unwrap(),
        before
    );
}
/// A competing final directory is never overwritten, even after all validations.
#[test]
fn service_installation_publish_race_preserves_existing_destination() {
    valid();
    let (_d, _p, a) = fixture();
    let path = a.root_directory().to_owned();
    let e = install::install_with_hook(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::Create,
        Arc::new(move |phase| {
            if phase == install::FixturePhase::ReadyToPublish {
                fs::create_dir(&path).unwrap();
                fs::write(Path::new(&path).join("keep"), b"unchanged").unwrap();
            }
            Ok(())
        }),
    )
    .unwrap_err();
    assert_ne!(e.progress.published, Publication::Durable);
    assert_eq!(
        fs::read(Path::new(a.root_directory()).join("keep")).unwrap(),
        b"unchanged"
    );
    assert!(stage(&a).exists());
}
/// Resume rejects symlinks/hardlinks/modes before SQLite follows a sidecar or DB.
#[test]
fn service_installation_unsafe_journal_and_sidecar_refuse_without_touching_target() {
    for variant in 0..4 {
        valid();
        let (d, _p, a) = fixture();
        let e = install::install_with_hook(
            &install::Context::new(),
            &a,
            &a.pin().unwrap(),
            install::Mode::Create,
            Arc::new(|phase| {
                if phase == install::FixturePhase::IntentDurable {
                    Err(install::Error::Interrupted)
                } else {
                    Ok(())
                }
            }),
        )
        .unwrap_err();
        assert_eq!(e.progress.intent, Publication::Durable);
        let outside = d.path().join("outside");
        fs::write(&outside, b"OUTSIDE").unwrap();
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o600)).unwrap();
        let db = stage(&a).join("business/journal.sqlite");
        match variant {
            0 => symlink(&outside, &db).unwrap(),
            1 => fs::hard_link(&outside, &db).unwrap(),
            2 => {
                fs::write(&db, b"").unwrap();
                fs::set_permissions(&db, fs::Permissions::from_mode(0o644)).unwrap();
            }
            _ => symlink(&outside, stage(&a).join("business/journal.sqlite-wal")).unwrap(),
        };
        assert!(install::install(
            &install::Context::new(),
            &a,
            &a.pin().unwrap(),
            install::Mode::ResumeStage
        )
        .is_err());
        assert_eq!(fs::read(&outside).unwrap(), b"OUTSIDE");
        assert!(!Path::new(a.root_directory()).exists());
    }
}
/// Cancellation has no implicit rollback and a fresh explicit resume can progress.
#[test]
fn service_installation_cancelled_after_intent_requires_explicit_resume() {
    valid();
    let (_d, _p, a) = fixture();
    let ctx = install::Context::new();
    let cancel = ctx.clone();
    let e = install::install_with_hook(
        &ctx,
        &a,
        &a.pin().unwrap(),
        install::Mode::Create,
        Arc::new(move |phase| {
            if phase == install::FixturePhase::IntentDurable {
                cancel.cancel()
            }
            Ok(())
        }),
    )
    .unwrap_err();
    assert_eq!(e.progress.intent, Publication::Durable);
    assert_eq!(e.progress.published, Publication::NotAttempted);
    assert!(!Path::new(a.root_directory()).exists());
    assert!(install::install(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::ResumeStage
    )
    .is_ok());
}

/// Independent fixture frame: changing an opaque event is allowed as data, never
/// treated as authority until the real journals and semantic checks run.
fn changed_business(a: &artifact::ServiceArtifact) -> artifact::ServiceArtifact {
    use sha2::{Digest, Sha256};
    let mut event = a.business_initial().to_vec();
    let needle = b"\"signature\":\"";
    let at = event
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap()
        + needle.len();
    event[at] = if event[at] == b'A' { b'B' } else { b'A' };
    let mut doc = a.document().clone();
    doc.control.authority_head.digest = format!("{:x}", Sha256::digest(&event));
    let doc = swarm_enrollment_bootstrap::encode_authenticated(&doc).unwrap();
    let mut seed = Zeroizing::new(Vec::with_capacity(32));
    a.write_personal_seed(&mut *seed).unwrap();
    let capacity = b"swarm-service-installation-v1\n".len()
        + 32
        + 24
        + doc.len()
        + event.len()
        + a.personal_initial().len();
    let mut raw = Zeroizing::new(Vec::with_capacity(capacity));
    raw.extend_from_slice(b"swarm-service-installation-v1\n");
    raw.extend_from_slice(&seed);
    for bytes in [doc.as_slice(), event.as_slice(), a.personal_initial()] {
        raw.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        raw.extend_from_slice(bytes)
    }
    let pin = format!("{:x}", Sha256::digest(&raw));
    artifact::decode(&raw, &pin).unwrap()
}
/// Signed-data binding alone cannot publish an invalid signature or expired grant.
#[test]
fn service_installation_bad_signature_and_expired_grant_never_publish() {
    valid();
    let (_d, _p, a) = fixture();
    let bad = changed_business(&a);
    let e = install::install(
        &install::Context::new(),
        &bad,
        &bad.pin().unwrap(),
        install::Mode::Create,
    )
    .unwrap_err();
    assert_eq!(e.progress.published, Publication::NotAttempted);
    assert!(!Path::new(a.root_directory()).exists());
    valid();
    let (d, _p, _a) = fixture();
    let mut r: p::Request = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    r.root_directory = d.path().join("expired").to_str().unwrap().into();
    r.service_uid = rustix::process::geteuid().as_raw();
    r.controller_uid = if r.service_uid == 0 {
        1
    } else {
        !r.service_uid
    };
    r.grant.not_before = time::OffsetDateTime::now_utc() - time::Duration::hours(2);
    r.grant.expires_at = time::OffsetDateTime::now_utc() - time::Duration::hours(1);
    let a = artifact::from_prepared(&p::prepare(&r).unwrap()).unwrap();
    let e = install::install(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::Create,
    )
    .unwrap_err();
    assert_eq!(e.progress.business, install::JournalProgress::Acknowledged);
    assert_eq!(e.progress.personal, install::JournalProgress::Acknowledged);
    assert_eq!(e.progress.published, Publication::NotAttempted);
    assert!(!Path::new(a.root_directory()).exists());
}
/// A replacement artifact cannot adopt a previous stage's intent or advance it.
#[test]
fn service_installation_changed_material_refuses_before_stage_acknowledgement() {
    valid();
    let (_d, _p, a) = fixture();
    install::install_with_hook(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::Create,
        Arc::new(|p| {
            if p == install::FixturePhase::IntentDurable {
                Err(install::Error::Interrupted)
            } else {
                Ok(())
            }
        }),
    )
    .unwrap_err();
    let other = artifact::from_prepared(&material(Path::new(a.root_directory()))).unwrap();
    let before = fs::read(stage(&a).join("service-installation-v1.intent")).unwrap();
    let e = install::install(
        &install::Context::new(),
        &other,
        &other.pin().unwrap(),
        install::Mode::ResumeStage,
    )
    .unwrap_err();
    assert_ne!(e.progress.stage, Publication::Durable);
    assert_eq!(e.progress.intent, Publication::NotAttempted);
    assert_eq!(e.progress.business, install::JournalProgress::NotAttempted);
    assert_eq!(
        before,
        fs::read(stage(&a).join("service-installation-v1.intent")).unwrap()
    );
    assert!(!Path::new(a.root_directory()).exists());
    assert!(install::install(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::ResumeStage
    )
    .is_ok());
}
/// A pristine pre-intent stage can resume, but any unexplained data prevents that.
#[test]
fn service_installation_pristine_stage_resume_never_adopts_existing_data() {
    for inert in [true, false] {
        valid();
        let (_d, _p, a) = fixture();
        let path = stage(&a);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        for name in [
            "identity",
            "business",
            "personal",
            "configuration",
            "configuration-witness",
            "personal-witness",
        ] {
            fs::create_dir(path.join(name)).unwrap();
            fs::set_permissions(path.join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
        if !inert {
            fs::write(path.join("business/journal.sqlite"), b"UNEXPLAINED").unwrap();
            fs::set_permissions(
                path.join("business/journal.sqlite"),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        let result = install::install(
            &install::Context::new(),
            &a,
            &a.pin().unwrap(),
            install::Mode::ResumeStage,
        );
        assert_eq!(result.is_ok(), inert);
        if !inert {
            assert!(!path.join("service-installation-v1.intent").exists());
            assert_eq!(
                fs::read(path.join("business/journal.sqlite")).unwrap(),
                b"UNEXPLAINED"
            );
        }
    }
}
/// Replacing a retained directory under the same path refuses before publication.
#[test]
fn service_installation_directory_replacement_before_publish_refuses() {
    valid();
    let (_d, _p, a) = fixture();
    let path = stage(&a);
    let hookpath = path.clone();
    let e = install::install_with_hook(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::Create,
        Arc::new(move |phase| {
            if phase == install::FixturePhase::ReadyToPublish {
                fs::rename(hookpath.join("business"), hookpath.join("business-old")).unwrap();
                fs::create_dir(hookpath.join("business")).unwrap();
                fs::set_permissions(hookpath.join("business"), fs::Permissions::from_mode(0o700))
                    .unwrap();
            }
            Ok(())
        }),
    )
    .unwrap_err();
    assert_eq!(e.progress.published, Publication::NotAttempted);
    assert!(!Path::new(a.root_directory()).exists());
    assert!(path.join("business-old/journal.sqlite").exists());
}

/// Even incomplete structure cannot be repaired before its stored pin is checked.
#[test]
fn service_installation_mismatched_intent_does_not_recreate_missing_child() {
    valid();
    let (_d, _p, a) = fixture();
    install::install_with_hook(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::Create,
        Arc::new(|phase| {
            if phase == install::FixturePhase::IntentDurable {
                Err(install::Error::Interrupted)
            } else {
                Ok(())
            }
        }),
    )
    .unwrap_err();
    let missing = stage(&a).join("configuration");
    fs::remove_dir(&missing).unwrap();
    let other = artifact::from_prepared(&material(Path::new(a.root_directory()))).unwrap();
    assert!(install::install(
        &install::Context::new(),
        &other,
        &other.pin().unwrap(),
        install::Mode::ResumeStage
    )
    .is_err());
    assert!(
        !missing.exists(),
        "repaired structure before validating stored intent"
    );
    assert!(install::install(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::ResumeStage
    )
    .is_ok());
}

/// Actual descriptors close before injected EIO; durability and both independent
/// failures survive instead of relabeling a committed root as absent.
#[test]
fn service_installation_close_errors_keep_durable_root_and_primary_refusal() {
    valid();
    let (_d, _p, a) = fixture();
    let hook = Arc::new(|phase| {
        if phase == install::FixturePhase::TreeClosed {
            Err(install::Error::Io {
                phase: install::IoPhase::Close,
                cause: std::io::Error::other("PRIVATE_CLOSE_CAUSE"),
            })
        } else {
            Ok(())
        }
    });
    let e = install::install_with_hook(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::Create,
        hook.clone(),
    )
    .unwrap_err();
    assert_eq!(e.progress.published, Publication::Durable);
    assert!(!format!("{e:?} {e}").contains("PRIVATE_CLOSE_CAUSE"));
    let r = install::install(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::RecoverPublished,
    )
    .unwrap();
    assert_eq!(r.progress.published, Publication::Durable);
    let (_d, _p, a) = fixture();
    let bad = changed_business(&a);
    let e = install::install_with_hook(
        &install::Context::new(),
        &bad,
        &bad.pin().unwrap(),
        install::Mode::Create,
        hook,
    )
    .unwrap_err();
    assert_eq!(e.progress.published, Publication::NotAttempted);
    match e.cause {
        install::Error::Joined { cause, cleanup } => {
            assert!(matches!(*cause, install::Error::State(_)));
            assert!(matches!(
                *cleanup,
                install::Error::Io {
                    phase: install::IoPhase::Close,
                    ..
                }
            ));
        }
        _ => panic!("primary and independent close refusal not retained"),
    }
}
/// Initial-only published reconciliation never resets an account that advanced.
#[test]
fn service_installation_initial_recovery_refuses_advanced_account_without_reset() {
    valid();
    let (_d, p, a) = fixture();
    let r = install::install(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::Create,
    )
    .unwrap();
    let ctx = a::authenticated_control::Context::new();
    let life = a::authenticated_startup::open_lifecycle(
        &swarm_enrollment_state::Context::new(),
        &r.bootstrap_path,
        &r.bootstrap_pin,
        Path::new(&r.personal_seed_path),
    )
    .unwrap();
    life.initialize(&ctx).unwrap();
    let mut proof = Zeroizing::new(Vec::with_capacity(32));
    p.write_installation_proof(&mut *proof).unwrap();
    let account = life
        .enroll(
            &ctx,
            proof.as_slice().try_into().unwrap(),
            b"synthetic-advanced-account",
        )
        .unwrap();
    life.close().unwrap();
    let db = Path::new(a.root_directory()).join("personal/journal.sqlite");
    let before = fs::read(&db).unwrap();
    assert!(install::install(
        &install::Context::new(),
        &a,
        &a.pin().unwrap(),
        install::Mode::RecoverPublished
    )
    .is_err());
    assert_eq!(fs::read(&db).unwrap(), before);
    let life = a::authenticated_startup::open_lifecycle(
        &swarm_enrollment_state::Context::new(),
        &r.bootstrap_path,
        &r.bootstrap_pin,
        Path::new(&r.personal_seed_path),
    )
    .unwrap();
    assert_eq!(life.recover(&ctx).unwrap().head(), account.head());
    life.close().unwrap();
}
