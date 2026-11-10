//! Canonical data projection, not adoption, authentication, freshness or a permit.
use super::{identity_plan_rules as rules, model_wire};
use rusqlite::{functions::FunctionFlags, types::ValueRef, Connection};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;

/// Projection format only. A digest supplies neither adoption nor authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i64)]
pub enum IdentityPlanProjection {
    V1 = 1,
    V2 = 2,
}

pub enum IdentityPlanError {
    Json(serde_json::Error),
    Shape(model_wire::Error),
    Invalid,
}
impl IdentityPlanError {
    pub fn code(&self) -> &'static str {
        "agent_identity_plan_invalid"
    }
}
impl fmt::Display for IdentityPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for IdentityPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for IdentityPlanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            Self::Shape(e) => Some(e),
            Self::Invalid => None,
        }
    }
}
impl From<serde_json::Error> for IdentityPlanError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<model_wire::Error> for IdentityPlanError {
    fn from(e: model_wire::Error) -> Self {
        Self::Shape(e)
    }
}

enum Node {
    Scalar(Vec<u8>),
    Object(BTreeMap<String, usize>),
    Array(Vec<usize>),
}
struct Document(Vec<Node>);
impl Document {
    fn parse(raw: &[u8]) -> Result<Self, IdentityPlanError> {
        // Go UseNumber semantics: JSON syntax/depth is checked, but number
        // lexemes are retained, including huge exponents, -0 and integer precision.
        swarm_wire_json::decode::<serde::de::IgnoredAny>(raw)?;
        let boundaries = model_wire::Boundaries::new(raw)?;
        let mut document = Self(vec![Node::Scalar(b"null".to_vec())]);
        let mut pending = vec![(raw, 0)];
        while let Some((raw, index)) = pending.pop() {
            let raw = raw.trim_ascii();
            match raw.first() {
                Some(b'{') => {
                    let mut fields = BTreeMap::new();
                    let mut children = Vec::new();
                    for (key, value) in model_wire::object(raw, &boundaries)? {
                        let child = document.0.len();
                        document.0.push(Node::Scalar(b"null".to_vec()));
                        fields.insert(key, child);
                        children.push((value, child));
                    }
                    document.0[index] = Node::Object(fields);
                    pending.extend(children.into_iter().rev());
                }
                Some(b'[') => {
                    let mut fields = Vec::new();
                    let mut children = Vec::new();
                    for value in model_wire::array(raw, &boundaries)? {
                        let child = document.0.len();
                        document.0.push(Node::Scalar(b"null".to_vec()));
                        fields.push(child);
                        children.push((value, child));
                    }
                    document.0[index] = Node::Array(fields);
                    pending.extend(children.into_iter().rev());
                }
                Some(b'"') => {
                    let string: String = swarm_wire_json::decode(raw)?;
                    document.0[index] = Node::Scalar(swarm_wire_json::encode(&string)?);
                }
                Some(_) => document.0[index] = Node::Scalar(raw.to_vec()),
                None => return Err(IdentityPlanError::Invalid),
            }
        }
        Ok(document)
    }
    fn object(&self, index: usize) -> Result<&BTreeMap<String, usize>, IdentityPlanError> {
        match &self.0[index] {
            Node::Object(v) => Ok(v),
            _ => Err(IdentityPlanError::Invalid),
        }
    }
    fn omit(&mut self, index: usize, keys: &[&str]) -> Result<(), IdentityPlanError> {
        let Node::Object(ref mut object) = self.0[index] else {
            return Err(IdentityPlanError::Invalid);
        };
        for &key in keys {
            object.remove(key);
        }
        Ok(())
    }
    fn null(&self, index: usize) -> bool {
        matches!(&self.0[index],Node::Scalar(v)if v==b"null")
    }
    fn objects(&self, index: usize) -> Result<Vec<usize>, IdentityPlanError> {
        if self.null(index) {
            return Ok(Vec::new());
        }
        let Node::Array(values) = &self.0[index] else {
            return Err(IdentityPlanError::Invalid);
        };
        let mut objects = Vec::new();
        for &id in values {
            if !self.null(id) {
                self.object(id)?;
                objects.push(id);
            }
        }
        Ok(objects)
    }
    fn apply(
        &mut self,
        index: usize,
        owner: &str,
        projection: IdentityPlanProjection,
    ) -> Result<(), IdentityPlanError> {
        let keys = rules::OMITS
            .iter()
            .find(|(role, _)| *role == owner)
            .ok_or(IdentityPlanError::Invalid)?
            .1;
        self.omit(index, keys)?;
        if owner == "task" && projection == IdentityPlanProjection::V2 {
            // Real Task producer tags. Keep the legacy exclusions as well;
            // changing v1 would silently invalidate persisted scopes.
            self.omit(index, &["automatic_validation", "manual_override"])?;
        }
        for &(role, key, keys) in rules::NESTED {
            if role != owner {
                continue;
            }
            let child = self.object(index)?.get(key).copied();
            if let Some(child) = child {
                if !self.null(child) {
                    self.omit(child, keys)?;
                }
            }
        }
        Ok(())
    }
    fn project(&mut self, projection: IdentityPlanProjection) -> Result<usize, IdentityPlanError> {
        self.apply(0, "object", projection)?;
        if let Some(tasks) = self.object(0)?.get("tasks").copied() {
            for task in self.objects(tasks)? {
                self.apply(task, "task", projection)?;
            }
        }
        if let Some(planning) = self.object(0)?.get("planning").copied() {
            if !self.null(planning) {
                self.apply(planning, "planning", projection)?;
                let scopes = *self
                    .object(planning)?
                    .get("scopes")
                    .ok_or(IdentityPlanError::Invalid)?;
                for scope in self.objects(scopes)? {
                    self.apply(scope, "scope", projection)?;
                }
            }
        }
        let version = self.0.len();
        self.0
            .push(Node::Scalar((projection as i64).to_string().into_bytes()));
        let root = self.0.len();
        self.0.push(Node::Object(BTreeMap::from([
            ("projection_version".into(), version),
            ("work".into(), 0),
        ])));
        Ok(root)
    }
    fn canonical(&self, root: usize) -> Result<Vec<u8>, IdentityPlanError> {
        enum Token {
            Node(usize),
            Bytes(Vec<u8>),
            Literal(&'static [u8]),
        }
        let mut output = Vec::new();
        let mut pending = vec![Token::Node(root)];
        while let Some(token) = pending.pop() {
            match token {
                Token::Bytes(v) => output.extend(v),
                Token::Literal(v) => output.extend_from_slice(v),
                Token::Node(index) => match &self.0[index] {
                    Node::Scalar(v) => output.extend_from_slice(v),
                    Node::Array(values) => {
                        pending.push(Token::Literal(b"]"));
                        for (i, &v) in values.iter().enumerate().rev() {
                            pending.push(Token::Node(v));
                            if i > 0 {
                                pending.push(Token::Literal(b","));
                            }
                        }
                        pending.push(Token::Literal(b"["));
                    }
                    Node::Object(fields) => {
                        pending.push(Token::Literal(b"}"));
                        for (i, (key, &v)) in fields.iter().enumerate().rev() {
                            pending.push(Token::Node(v));
                            pending.push(Token::Literal(b":"));
                            pending.push(Token::Bytes(swarm_wire_json::encode(key)?));
                            if i > 0 {
                                pending.push(Token::Literal(b","));
                            }
                        }
                        pending.push(Token::Literal(b"{"));
                    }
                },
            }
        }
        Ok(output)
    }
}
/// Hash only the recorded definition projection. This authenticates no actor.
pub fn identity_plan_digest(raw: &[u8]) -> Result<String, IdentityPlanError> {
    identity_plan_digest_for(raw, IdentityPlanProjection::V1)
}
/// Explicit version selection, including v1 historical compatibility.
pub fn identity_plan_digest_for(
    raw: &[u8],
    projection: IdentityPlanProjection,
) -> Result<String, IdentityPlanError> {
    let mut document = Document::parse(raw)?;
    let root = document.project(projection)?;
    let canonical = document.canonical(root)?;
    // Go decodes its versioned wrapper with UseNumber before final hashing.
    // That additional object counts toward the same JSON representation depth.
    swarm_wire_json::decode::<serde::de::IgnoredAny>(&canonical)?;
    Ok(Sha256::digest(canonical)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
/// Attach the deterministic function only. No schema, storage policy or rights change.
pub fn register_identity_plan_projection(connection: &Connection) -> rusqlite::Result<()> {
    debug_assert_eq!(rules::PROJECTION_VERSION, IdentityPlanProjection::V1 as i64);
    for (name, projection) in [
        (rules::UDF_NAME, IdentityPlanProjection::V1),
        ("swarm_identity_plan_digest_v2", IdentityPlanProjection::V2),
    ] {
        connection.create_scalar_function(
            name,
            1,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
            move |context| {
                let result = match context.get_raw(0) {
                    ValueRef::Blob(raw) | ValueRef::Text(raw) => {
                        identity_plan_digest_for(raw, projection)
                    }
                    _ => Err(IdentityPlanError::Invalid),
                };
                result.map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))
            },
        )?;
    }
    Ok(())
}
