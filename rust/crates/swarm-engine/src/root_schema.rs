//! Validate the application's SQL contract, not just SQLite's version tag.
//! Read-only comparison; never recreate a missing table or silently discard data.
use rusqlite::Connection;
use std::{collections::BTreeMap, error::Error, fmt};
pub(crate) const INITIAL_SCHEMA: &str = include_str!("initial_schema_37.sql");
const PROJECTION_UPGRADE: &str = include_str!("identity_plan_upgrade.sql");
const PROJECTION_FINISH: &str = include_str!("identity_plan_upgrade_finish.sql");
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootSchemaFailure {
    Sql,
    MissingObject,
    ObjectKind,
    ObjectDefinition,
    UnexpectedCoreObject,
    InvalidDefinition,
}
pub struct RootSchemaError {
    pub failure: RootSchemaFailure,
    pub object: Option<String>,
    cause: Option<rusqlite::Error>,
    pub cleanup: Option<Box<rusqlite::Error>>,
}
impl RootSchemaError {
    fn sql(cause: rusqlite::Error) -> Self {
        Self {
            failure: RootSchemaFailure::Sql,
            object: None,
            cause: Some(cause),
            cleanup: None,
        }
    }
    fn mismatch(failure: RootSchemaFailure, object: Option<String>) -> Self {
        Self {
            failure,
            object,
            cause: None,
            cleanup: None,
        }
    }
    pub fn code(&self) -> &'static str {
        if self.failure == RootSchemaFailure::Sql {
            "root-schema-inspection"
        } else {
            "root-schema-mismatch"
        }
    }
}
impl fmt::Display for RootSchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for RootSchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for RootSchemaError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|e| e as &(dyn Error + 'static))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    Word(String),
    QuotedIdentifier(String),
    String(String),
    Symbol(char),
}
fn tokens(sql: &str) -> Result<Vec<Token>, RootSchemaError> {
    let chars: Vec<_> = sql.chars().collect();
    let mut index = 0;
    let mut result = Vec::new();
    while index < chars.len() {
        let c = chars[index];
        if c.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if c == '-' && chars.get(index + 1) == Some(&'-') {
            index += 2;
            while index < chars.len() && chars[index] != '\n' {
                index += 1
            }
            continue;
        }
        if c == '/' && chars.get(index + 1) == Some(&'*') {
            index += 2;
            while index < chars.len()
                && !(chars[index] == '*' && chars.get(index + 1) == Some(&'/'))
            {
                index += 1
            }
            if index == chars.len() {
                return Err(RootSchemaError::mismatch(
                    RootSchemaFailure::InvalidDefinition,
                    None,
                ));
            }
            index += 2;
            continue;
        }
        if matches!(c, '\'' | '"' | '`' | '[') {
            let ending = if c == '[' { ']' } else { c };
            index += 1;
            let mut text = String::new();
            let mut closed = false;
            while index < chars.len() {
                let next = chars[index];
                index += 1;
                if next == ending {
                    if c != '[' && chars.get(index) == Some(&ending) {
                        text.push(ending);
                        index += 1
                    } else {
                        closed = true;
                        break;
                    }
                } else {
                    text.push(next)
                }
            }
            if !closed {
                return Err(RootSchemaError::mismatch(
                    RootSchemaFailure::InvalidDefinition,
                    None,
                ));
            }
            result.push(if c == '\'' {
                Token::String(text)
            } else {
                Token::QuotedIdentifier(text)
            });
            continue;
        }
        if c.is_ascii_alphanumeric() || c == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1
            }
            result.push(Token::Word(
                chars[start..index]
                    .iter()
                    .collect::<String>()
                    .to_ascii_lowercase(),
            ));
            continue;
        }
        result.push(Token::Symbol(c));
        index += 1;
    }
    Ok(result)
}
#[derive(Clone)]
struct Object {
    kind: String,
    table: String,
    definition: Vec<Token>,
}
fn objects(db: &Connection) -> Result<BTreeMap<(String, String), Object>, RootSchemaError> {
    // '_' in LIKE is a wildcard. Inspect the exact internal prefix instead, so
    // a user object named sqliteX... cannot hide an extra trigger on a core table.
    let mut statement=db.prepare("SELECT type,name,tbl_name,sql FROM main.sqlite_schema WHERE substr(lower(name),1,7)!='sqlite_' ORDER BY name").map_err(RootSchemaError::sql)?;
    let values = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(RootSchemaError::sql)?;
    let mut result = BTreeMap::new();
    for value in values {
        let (kind, name, table, definition) = value.map_err(RootSchemaError::sql)?;
        let definition = definition.ok_or_else(|| {
            RootSchemaError::mismatch(RootSchemaFailure::InvalidDefinition, Some(name.clone()))
        })?;
        let definition = tokens(&definition).map_err(|mut e| {
            e.object = Some(name.clone());
            e
        })?;
        let key = (kind.clone(), name.to_ascii_lowercase());
        if result
            .insert(
                key,
                Object {
                    kind,
                    table: table.to_ascii_lowercase(),
                    definition,
                },
            )
            .is_some()
        {
            return Err(RootSchemaError::mismatch(
                RootSchemaFailure::InvalidDefinition,
                Some(name),
            ));
        }
    }
    Ok(result)
}
pub(crate) fn verify_root_schema(db: &Connection) -> Result<(), RootSchemaError> {
    let actual = objects(db)?;
    let expected = Connection::open_in_memory().map_err(RootSchemaError::sql)?;
    let checked = (|| {
        expected
            .busy_timeout(std::time::Duration::ZERO)
            .map_err(RootSchemaError::sql)?;
        crate::register_identity_plan_projection(&expected).map_err(RootSchemaError::sql)?;
        expected
            .execute_batch(INITIAL_SCHEMA)
            .map_err(RootSchemaError::sql)?;
        // Projection v2 is an existing explicit component migration, independent
        // of user_version=37. Its complete immutable history/schema is required.
        if actual.contains_key(&(
            "table".into(),
            "agent_identity_plan_projection_history".into(),
        )) {
            expected
                .execute_batch(PROJECTION_UPGRADE)
                .map_err(RootSchemaError::sql)?;
            expected
                .execute_batch(PROJECTION_FINISH)
                .map_err(RootSchemaError::sql)?;
        }
        let required = objects(&expected)?;
        // SQLite has separate object namespaces. Match (kind,name), preserving
        // extra triggers even when their name equals a required table's name.
        let ordered = required
            .iter()
            .filter(|((kind, _), _)| kind == "table")
            .chain(required.iter().filter(|((kind, _), _)| kind != "table"));
        for (key, wanted) in ordered {
            let found = actual.get(key).ok_or_else(|| {
                RootSchemaError::mismatch(
                    if actual.keys().any(|(_, name)| name == &key.1) {
                        RootSchemaFailure::ObjectKind
                    } else {
                        RootSchemaFailure::MissingObject
                    },
                    Some(key.1.clone()),
                )
            })?;
            if found.table != wanted.table {
                return Err(RootSchemaError::mismatch(
                    RootSchemaFailure::ObjectKind,
                    Some(key.1.clone()),
                ));
            }
        }
        for (key, wanted) in &required {
            let found = actual.get(key).ok_or_else(|| {
                RootSchemaError::mismatch(RootSchemaFailure::MissingObject, Some(key.1.clone()))
            })?;
            if found.definition != wanted.definition {
                return Err(RootSchemaError::mismatch(
                    RootSchemaFailure::ObjectDefinition,
                    Some(key.1.clone()),
                ));
            }
        }
        for (key, found) in &actual {
            if !required.contains_key(key)
                && matches!(found.kind.as_str(), "index" | "trigger")
                && required.contains_key(&("table".into(), found.table.clone()))
            {
                return Err(RootSchemaError::mismatch(
                    RootSchemaFailure::UnexpectedCoreObject,
                    Some(key.1.clone()),
                ));
            }
        }
        Ok(())
    })();
    match (checked, expected.close()) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) => Err(e),
        (Ok(()), Err((_, e))) => Err(RootSchemaError::sql(e)),
        (Err(mut primary), Err((_, cleanup))) => {
            primary.cleanup = Some(Box::new(cleanup));
            Err(primary)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ddl_comparison_ignores_layout_and_comments_but_preserves_literals_and_quoting() {
        assert_eq!(
            tokens("CREATE TABLE x(a TEXT DEFAULT 'Case  sensitive')").unwrap(),
            tokens("create /* note */ table x ( a text -- note\n default 'Case  sensitive' )")
                .unwrap()
        );
        assert_ne!(
            tokens("DEFAULT 'Case sensitive'").unwrap(),
            tokens("DEFAULT 'case sensitive'").unwrap()
        );
        assert_ne!(
            tokens("DEFAULT CURRENT_TIMESTAMP").unwrap(),
            tokens("DEFAULT \"CURRENT_TIMESTAMP\"").unwrap()
        );
        assert_ne!(
            tokens("CHECK(a>0)").unwrap(),
            tokens("CHECK(a>=0)").unwrap()
        );
        assert_eq!(
            tokens("SELECT 'one''two'").unwrap(),
            vec![
                Token::Word("select".into()),
                Token::String("one'two".into())
            ]
        );
        for broken in ["'unclosed", "\"unclosed", "/* unclosed"] {
            assert!(tokens(broken).is_err())
        }
    }
}
