use ed25519_dalek::SigningKey;
use swarm_enrollment_authority::{
    credentials::Parameters,
    personal::{self as p, Bootstrap, Expected, Source, BOOTSTRAP_ID},
};
use swarm_enrollment_state::{sign_change, BoundJournal, Context, Head, Journal};
use swarm_identity_contract::Principal;
/// Disposable independently pinned personal journal fixture, with no live authority.
pub struct Fixture {
    pub _dir: tempfile::TempDir,
    pub path: std::path::PathBuf,
    pub key: SigningKey,
    pub expected: Expected,
    pub head: Head,
    pub source: Source,
    pub proof: [u8; 32],
}
impl Fixture {
    /// Independent fixture pins/proof and real signed SQLite journal, never a human enrollment.
    pub fn new() -> Self {
        Self::with_generation(1)
    }
    /// Explicit positive personal-generation fixture, independent from the
    /// journal's first sequence. Invalid input panics before dependent checks.
    /// The default fixture remains generation1 for all existing consumers.
    pub fn with_generation(generation: i64) -> Self {
        assert!(generation > 0);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("personal.sqlite");
        let key = SigningKey::from_bytes(&[41; 32]);
        let expected = Expected {
            installation_digest: "a".repeat(64),
            principal: Principal {
                id: "human-1".into(),
                kind: "human".into(),
                issuer: "urn:swarm:personal:fixture".into(),
                subject: "stable-human".into(),
            },
            generation,
        };
        let proof = [42; 32];
        let bootstrap = Bootstrap {
            schema_version: 1,
            installation_digest: expected.installation_digest.clone(),
            principal: expected.principal.clone(),
            generation,
            parameters: Parameters {
                memory_kib: 32,
                passes: 1,
                parallelism: 1,
            },
            proof_commitment: p::commitment(&expected.installation_digest, &proof).unwrap(),
        };
        let j = Journal::open(
            &Context::new(),
            &path,
            &key.verifying_key().to_bytes(),
            [43; 32],
        )
        .unwrap();
        let raw = sign_change(
            &key,
            [43; 32],
            &Head::default(),
            "enroll",
            BOOTSTRAP_ID,
            &p::encode_bootstrap(&bootstrap).unwrap(),
        )
        .unwrap();
        let head = j.apply(&Context::new(), &Head::default(), &raw).unwrap();
        j.close().unwrap();
        let source = Source::open(
            &Context::new(),
            &path,
            &key.verifying_key().to_bytes(),
            [43; 32],
            expected.clone(),
        )
        .unwrap();
        Self {
            _dir: dir,
            path,
            key,
            expected,
            head,
            source,
            proof,
        }
    }
    /// Return a separately opened real writer for contention/revocation tests.
    pub fn bound(&self) -> BoundJournal {
        BoundJournal::open(
            &Context::new(),
            &self.path,
            &self.key.verifying_key().to_bytes(),
            [43; 32],
        )
        .unwrap()
    }
}
