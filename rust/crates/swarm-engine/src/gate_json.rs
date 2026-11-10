//! Iterative UseNumber JSON arena for scoring. Identity projection stays separate.
use crate::model_wire::{self, Boundaries};
use std::{collections::BTreeMap, error::Error, fmt};
pub(crate) enum Node {
    Scalar(Vec<u8>),
    Object(BTreeMap<String, usize>),
    Array(Vec<usize>),
}
pub(crate) struct Document {
    pub nodes: Vec<Node>,
}
pub(crate) struct JsonError {
    cause: Box<dyn Error + Send + Sync>,
}
impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("gate-json")
    }
}
impl fmt::Debug for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for JsonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}
impl From<serde_json::Error> for JsonError {
    fn from(e: serde_json::Error) -> Self {
        Self { cause: Box::new(e) }
    }
}
impl From<model_wire::Error> for JsonError {
    fn from(e: model_wire::Error) -> Self {
        Self { cause: Box::new(e) }
    }
}
fn quote(text: &str) -> Result<Vec<u8>, JsonError> {
    // Go Encoder.SetEscapeHTML(false) still escapes the JS line separators.
    let text = serde_json::to_string(text)?;
    Ok(text
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
        .into_bytes())
}
impl Document {
    pub fn parse(raw: &[u8]) -> Result<Self, JsonError> {
        swarm_wire_json::decode::<serde::de::IgnoredAny>(raw)?;
        let boundaries = Boundaries::new(raw)?;
        let mut out = Self {
            nodes: vec![Node::Scalar(b"null".to_vec())],
        };
        let mut pending = vec![(raw, 0)];
        while let Some((raw, index)) = pending.pop() {
            let raw = raw.trim_ascii();
            match raw.first() {
                Some(b'{') => {
                    let mut map = BTreeMap::new();
                    for (key, value) in model_wire::object(raw, &boundaries)? {
                        let child = out.nodes.len();
                        out.nodes.push(Node::Scalar(b"null".to_vec()));
                        map.insert(key, child);
                        pending.push((value, child));
                    }
                    out.nodes[index] = Node::Object(map);
                }
                Some(b'[') => {
                    let mut array = Vec::new();
                    for value in model_wire::array(raw, &boundaries)? {
                        let child = out.nodes.len();
                        out.nodes.push(Node::Scalar(b"null".to_vec()));
                        array.push(child);
                        pending.push((value, child));
                    }
                    out.nodes[index] = Node::Array(array);
                }
                Some(b'"') => {
                    let string: String = swarm_wire_json::decode(raw)?;
                    out.nodes[index] = Node::Scalar(quote(&string)?);
                }
                Some(_) => out.nodes[index] = Node::Scalar(raw.to_vec()),
                None => {
                    return Err(serde_json::from_slice::<serde_json::Value>(raw)
                        .unwrap_err()
                        .into())
                }
            }
        }
        Ok(out)
    }
    pub fn object(&self, index: usize) -> Option<&BTreeMap<String, usize>> {
        match &self.nodes[index] {
            Node::Object(map) => Some(map),
            _ => None,
        }
    }
    pub fn array(&self, index: usize) -> Option<&[usize]> {
        match &self.nodes[index] {
            Node::Array(v) => Some(v),
            _ => None,
        }
    }
    pub fn get(&self, index: usize, key: &str) -> Option<usize> {
        self.object(index)?.get(key).copied()
    }
    pub fn null(&self, index: usize) -> bool {
        matches!(&self.nodes[index],Node::Scalar(raw)if raw==b"null")
    }
    pub fn text(&self, index: usize) -> Option<String> {
        match &self.nodes[index] {
            Node::Scalar(raw) if raw.first() == Some(&b'"') => swarm_wire_json::decode(raw).ok(),
            _ => None,
        }
    }
    pub fn boolean(&self, index: usize) -> Option<bool> {
        match &self.nodes[index] {
            Node::Scalar(raw) if raw == b"true" => Some(true),
            Node::Scalar(raw) if raw == b"false" => Some(false),
            _ => None,
        }
    }
    pub fn number(&self, index: usize) -> Option<f64> {
        match &self.nodes[index] {
            Node::Scalar(raw)
                if raw
                    .first()
                    .is_some_and(|b| *b == b'-' || b.is_ascii_digit()) =>
            {
                let n = serde_json::from_slice::<f64>(raw).ok()?;
                if n.is_finite() && n >= 0.0 {
                    Some(n)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    pub fn push_array(&mut self, values: Vec<usize>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node::Array(values));
        id
    }
    pub fn push_text(&mut self, text: &str) -> Result<usize, JsonError> {
        let id = self.nodes.len();
        self.nodes.push(Node::Scalar(quote(text)?));
        Ok(id)
    }
    pub fn push_object(&mut self, fields: BTreeMap<String, usize>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node::Object(fields));
        id
    }
    pub fn canonical(&self, root: usize) -> Result<Vec<u8>, JsonError> {
        enum Token {
            Node(usize),
            Bytes(Vec<u8>),
            Literal(&'static [u8]),
        }
        let mut out = Vec::new();
        let mut pending = vec![Token::Node(root)];
        while let Some(token) = pending.pop() {
            match token {
                Token::Bytes(v) => out.extend(v),
                Token::Literal(v) => out.extend_from_slice(v),
                Token::Node(id) => match &self.nodes[id] {
                    Node::Scalar(raw) => out.extend_from_slice(raw),
                    Node::Array(values) => {
                        pending.push(Token::Literal(b"]"));
                        for (i, &v) in values.iter().enumerate().rev() {
                            pending.push(Token::Node(v));
                            if i > 0 {
                                pending.push(Token::Literal(b", "));
                            }
                        }
                        pending.push(Token::Literal(b"["));
                    }
                    Node::Object(fields) => {
                        pending.push(Token::Literal(b"}"));
                        for (i, (key, &v)) in fields.iter().enumerate().rev() {
                            pending.push(Token::Node(v));
                            pending.push(Token::Literal(b": "));
                            pending.push(Token::Bytes(quote(key)?));
                            if i > 0 {
                                pending.push(Token::Literal(b", "));
                            }
                        }
                        pending.push(Token::Literal(b"{"));
                    }
                },
            }
        }
        Ok(out)
    }
}
