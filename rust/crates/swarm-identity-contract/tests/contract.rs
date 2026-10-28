use swarm_identity_contract::{digest, name, text, Permission, Principal, Scope};

fn human() -> Principal {
    Principal {
        id: "human-1".into(),
        kind: "human".into(),
        issuer: "https://[::1]/issuer".into(),
        subject: "employee[eu]".into(),
    }
}

#[test]
fn name_preserves_existing_ascii_and_byte_boundaries() {
    for value in ["work-A_01", &"a".repeat(100)] {
        assert!(name(value.as_bytes()), "{value:?}");
    }
    for value in ["", "../work", "équipe", "work\0", &"a".repeat(101)] {
        assert!(!name(value.as_bytes()), "{value:?}");
    }
    assert!(!name(&[0xff]));
}

#[test]
fn text_preserves_utf8_opaque_content_and_rejects_controls_and_padding() {
    for value in [
        "subject[eu]",
        "équipe",
        "a b",
        "a\u{2003}b",
        "\u{200b}",
        "a\u{feff}",
    ] {
        assert!(text(value.as_bytes()), "{value:?}");
    }
    for value in [
        "",
        " a",
        "a ",
        "\u{a0}a",
        "a\u{2003}",
        "a\0b",
        "a\u{85}b",
        "\n",
    ] {
        assert!(!text(value.as_bytes()), "{value:?}");
    }
    assert!(!text(&[0xc0, 0xaf]));
    assert!(!text(&[0xff]));
}

#[test]
fn digest_requires_exactly_32_bytes_in_lower_hex() {
    assert!(digest("0123456789abcdef".repeat(4).as_bytes()));
    for value in [
        "",
        &"0".repeat(63),
        &"0".repeat(65),
        &"A".repeat(64),
        &"g".repeat(64),
    ] {
        assert!(!digest(value.as_bytes()));
    }
}

#[test]
fn human_and_service_data_do_not_become_local_principals() {
    let mut principal = human();
    assert!(principal.valid());
    principal.kind = "service".into();
    assert!(principal.valid());
    principal.kind = "local".into();
    assert!(!principal.valid());
    principal.issuer = "urn:swarm:local".into();
    principal.subject = principal.id.clone();
    assert!(principal.valid());
    principal.kind = "human".into();
    assert!(!principal.valid());
    principal = human();
    principal.kind = "admin".into();
    assert!(!principal.valid());
}

#[test]
fn principal_valid_control_precedes_each_changed_field() {
    for field in ["id", "issuer", "subject"] {
        let mut principal = human();
        assert!(principal.valid());
        match field {
            "id" => principal.id = "../human".into(),
            "issuer" => principal.issuer = " issuer".into(),
            "subject" => principal.subject = "subject\0".into(),
            _ => unreachable!(),
        }
        assert!(!principal.valid());
    }
}

#[test]
fn scope_requires_exact_work_positive_version_and_canonical_digest() {
    let scope = Scope {
        work_id: "work-1".into(),
        plan_version: 1,
        plan_digest: "a".repeat(64),
    };
    assert!(scope.valid());
    for version in [0, -1, i64::MIN] {
        assert!(!Scope {
            plan_version: version,
            ..scope.clone()
        }
        .valid());
    }
    assert!(Scope {
        plan_version: i64::MAX,
        ..scope.clone()
    }
    .valid());
    assert!(!Scope {
        work_id: "../work".into(),
        ..scope.clone()
    }
    .valid());
    assert!(!Scope {
        plan_digest: "A".repeat(64),
        ..scope
    }
    .valid());
}

#[test]
fn permission_operations_and_exact_resources_are_not_globs() {
    for operation in [
        "agent.create",
        "memory.read",
        "memory.summarize",
        "memory.transmit",
        "tool.invoke",
        "effect.execute",
    ] {
        let permission = Permission {
            operation: operation.into(),
            resource: "memory/équipe".into(),
        };
        assert!(permission.valid());
        for resource in [
            "",
            "memory/*",
            "memory/?",
            "memory/[eu]",
            " memory",
            "memory\0",
        ] {
            assert!(!Permission {
                resource: resource.into(),
                ..permission.clone()
            }
            .valid());
        }
    }
    assert!(!Permission {
        operation: "memory.write".into(),
        resource: "memory/1".into()
    }
    .valid());
}
