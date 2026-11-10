//! Pure legacy field-guide reference predicate, proposed by GLM and compared
//! with retained Go. This performs no IO, version admission or authority check.

use std::collections::BTreeSet;

const FIELD_GUIDE_DIR: &str = ".claude/field-guide";

/// Équivalent du prédicat Go `filteredMemoryGuide(body, paths) != body`.
///
/// La fonction Go retire chaque ligne « gérée » puis recolle avec `"\n"` ;
/// `strings.Split`/`Join` sur `"\n"` étant un aller-retour exact, le corps
/// filtré diffère de l'original ssi au moins une ligne est gérée. On renvoie
/// donc `true` dès qu'une ligne est gérée, sans reconstruire la chaîne.
///
/// Une ligne est gérée si :
/// * elle contient un lien markdown `[texte](cible)` — pattern Go
///   `\[[^\]]*\]\(([^)]+)\)`, correspondances gauche→droite non chevauchantes
///   — dont la cible, après `TrimSpace` puis `Trim "<>"`, parses comme
///   référence `net/url` **sans scheme ni hôte** (sinon match ignoré), et
///   dont le chemin décodé, nettoyé comme `filepath.Clean`, vaut une entrée
///   de `paths` ou `filepath.Clean(filepath.Join(".claude/field-guide", c))` ;
/// * ou elle contient en clair une entrée de `paths`, ou la sous-chaîne
///   `"](Base(entrée))"` (boucle floue d'origine, volontairement large).
///
/// Hypothèses : séparateurs Unix (`filepath.ToSlash` = identité) ; entrées
/// de `paths` toutes à `true` côté Go ; UTF-8 valide (garanti par
/// `&str`/`String`). Fonction pure : aucun I/O, DB ou permission.
pub(crate) fn guide_references_managed(body: &str, paths: &BTreeSet<String>) -> bool {
    body.split('\n').any(|line| line_is_managed(line, paths))
}

/// Match local decoded links and the historical conservative text markers. Pure; no input mutation.
fn line_is_managed(line: &str, paths: &BTreeSet<String>) -> bool {
    for raw in markdown_link_targets(line) {
        // strings.Trim(strings.TrimSpace(match[1]), "<>")
        let t = raw.trim().trim_matches(|c| c == '<' || c == '>');
        if let Some(path) = go_url_path(t) {
            let target = clean(&path);
            if paths.contains(&target)
                || paths.contains(&clean(&format!("{FIELD_GUIDE_DIR}/{target}")))
            {
                return true;
            }
        }
    }
    paths
        .iter()
        .any(|p| line.contains(p.as_str()) || line.contains(&format!("]({})", base(p))))
}

/// Scan équivalent à `memoryGuideLinks.FindAllStringSubmatch(line, -1)` :
/// renvoie les groupes `([^)]+)`. Délimiteurs ASCII ⇒ scan par octets sûr en
/// UTF-8. Pour un `[` donné, `[^\]]*` ne peut s'arrêter qu'au premier `]`
/// suivant et `[^)]+` au premier `)` (aucun backtracking utile) : sémantique
/// « leftmost, non chevauchant, reprise après la fin du match » reproduite.
fn markdown_link_targets(line: &str) -> Vec<&str> {
    let b = line.as_bytes();
    let mut targets = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'[' {
            i += 1;
            continue;
        }
        let Some(rel) = b[i + 1..].iter().position(|&c| c == b']') else {
            break; // plus de ']' : plus aucun match possible
        };
        let close = i + 1 + rel;
        if b.get(close + 1) != Some(&b'(') {
            i += 1; // échec en i : le moteur Go reprend en i+1
            continue;
        }
        let start = close + 2;
        if start >= b.len() || b[start] == b')' {
            i += 1; // groupe [^)]+ vide : pas de match en i
            continue;
        }
        let Some(rel2) = b[start..].iter().position(|&c| c == b')') else {
            break; // plus de ')' : plus aucun match possible
        };
        let end = start + rel2;
        targets.push(&line[start..end]);
        i = end + 1; // FindAll reprend après la fin du match
    }
    targets
}

/// Sous-ensemble de `net/url.Parse` (viaRequest=false) : `Some(chemin décodé)`
/// ssi Parse réussit avec `Scheme == "" && Host == ""`, `None` sinon
/// (erreur de Parse, scheme présent ou hôte présent ⇒ `continue` Go).
/// Parse the scheme/authority-free subset needed by the retained Unix Go filter. Invalid escapes and remote references yield None; this is not a general URL parser.
fn go_url_path(raw: &str) -> Option<String> {
    // url.Parse coupe d'abord le fragment au premier '#'.
    let (u, frag) = match raw.find('#') {
        Some(i) => (&raw[..i], Some(&raw[i + 1..])),
        None => (raw, None),
    };
    // parse() : octets de contrôle interdits (partie pré-fragment seulement).
    if u.bytes().any(|c| c < 0x20 || c == 0x7f) {
        return None;
    }
    // setFragment() : échappement invalide dans le fragment ⇒ erreur.
    if let Some(f) = frag {
        if !has_valid_percent_escapes(f) {
            return None;
        }
    }
    if u == "*" {
        return Some("*".to_string());
    }
    // getScheme() : lettre en tête puis [A-Za-z0-9+.-]* puis ':'.
    // Tout scheme (chemin opaque ou hiérarchique) ⇒ Scheme != "" ⇒ ignoré.
    for (i, &c) in u.as_bytes().iter().enumerate() {
        match c {
            b':' => return None, // i == 0 : « missing protocol scheme » ; sinon scheme présent
            c if c.is_ascii_alphabetic() => {}
            b'0'..=b'9' | b'+' | b'-' | b'.' if i > 0 => {}
            _ => break, // pas de scheme
        }
    }
    // Coupure de la query (premier '?') ; ForceQuery ne change pas le chemin.
    let rest = match u.find('?') {
        Some(i) => &u[..i],
        None => u,
    };
    // RFC 3986 : premier segment d'un chemin relatif sans ':'.
    if !rest.starts_with('/') && rest.split('/').next().unwrap_or("").contains(':') {
        return None; // « first path segment in URL cannot contain colon »
    }
    // Autorité « //… » sauf « ///… » (net/url actuel) : hôte non vide ⇒ ignoré ;
    // hôte vide (« // », « //@/x ») ⇒ Go poursuit sur le chemin restant.
    if rest.starts_with("//") && !rest.starts_with("///") {
        let after = &rest[2..];
        let auth = match after.find('/') {
            Some(i) => &after[..i],
            None => after,
        };
        let (userinfo, host) = match auth.rfind('@') {
            Some(i) => (Some(&auth[..i]), &auth[i + 1..]),
            None => (None, auth),
        };
        if let Some(ui) = userinfo {
            if !valid_userinfo(ui) {
                return None; // « invalid userinfo » ⇒ match ignoré
            }
        }
        if !host.is_empty() {
            return None; // Host != "" (erreurs hôte/port ⇒ aussi ignoré)
        }
        return percent_decode_path(&after[auth.len()..]);
    }
    percent_decode_path(rest)
}

/// Validate permitted userinfo bytes and percent escapes before accepting an empty host. No decoding or remote identity is inferred.
fn valid_userinfo(s: &str) -> bool {
    if !has_valid_percent_escapes(s) {
        return false;
    }
    s.bytes().all(|c| {
        matches!(
            c,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
                | b'-' | b'.' | b'_' | b':' | b'~' | b'!' | b'$' | b'&' | b'\''
                | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b'%' | b'@'
        )
    })
}

/// Require two ASCII hex digits per percent marker; inspect bytes without decoding.
fn has_valid_percent_escapes(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            if i + 2 >= b.len() || !b[i + 1].is_ascii_hexdigit() || !b[i + 2].is_ascii_hexdigit() {
                return false;
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    true
}

/// setPath()/unescape(encodePath) : décodage %XX ('+' littéral). Échappement
/// invalide ou résultat non UTF-8 ⇒ None (Go conserverait des octets
/// arbitraires ; aucune entrée `String` ne peut alors égaler, même résultat).
/// Decode valid percent pairs into UTF-8; invalid or non-UTF8 paths cannot match stored Rust String paths.
fn percent_decode_path(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            if i + 2 >= b.len() {
                return None;
            }
            match (
                (b[i + 1] as char).to_digit(16),
                (b[i + 2] as char).to_digit(16),
            ) {
                (Some(h), Some(l)) => out.push((h * 16 + l) as u8),
                _ => return None,
            }
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Équivalent Unix de `filepath.Clean` (lexical : '//', '.', '..').
/// Apply retained Unix lexical filepath.Clean semantics; this is no filesystem resolution or containment check.
fn clean(path: &str) -> String {
    let rooted = path.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if rooted {
                    out.pop(); // au-dessus de la racine : '..' ignoré
                } else if out.is_empty() || out.last() == Some(&"..") {
                    out.push("..");
                } else {
                    out.pop();
                }
            }
            s => out.push(s),
        }
    }
    let mut res = String::new();
    if rooted {
        res.push('/');
    }
    res.push_str(&out.join("/"));
    if res.is_empty() {
        res.push('.');
    }
    res
}

/// Équivalent Unix de `filepath.Base`.
/// Return the Unix basename after lexical cleanup; no filesystem access or authority.
fn base(path: &str) -> &str {
    if path.is_empty() {
        return ".";
    }
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/";
    }
    match trimmed.rfind('/') {
        Some(i) => &trimmed[i + 1..],
        None => trimmed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// Témoin : au moins une ligne gérée ⇒ true ; corps intact ⇒ false.
    #[test]
    fn temoin_ligne_geree_vs_corps_intact() {
        let paths = set(&["guide.md"]);
        assert!(guide_references_managed(
            "intro\n[voir](guide.md)\nfin",
            &paths
        ));
        assert!(!guide_references_managed(
            "intro\n[sans lien] (rien)\nfin",
            &paths
        ));
        assert!(!guide_references_managed("", &paths));
    }

    /// Lien relatif géré ; liens « directs » (scheme ou hôte) ignorés.
    #[test]
    fn lien_relatif_gere_direct_ignore() {
        let paths = set(&["guides/a.md", "distant.md"]);
        assert!(guide_references_managed("[g](guides/a.md)", &paths));
        assert!(guide_references_managed("[g](./guides/a.md)", &paths));
        assert!(!guide_references_managed(
            "[g](https://exemple.org/x.md)",
            &paths
        ));
        assert!(!guide_references_managed("[g](//exemple.org/x.md)", &paths));
        assert!(!guide_references_managed("[g](mailto:x@y)", &paths));
    }

    /// Encodage URL : décodage %XX avant Clean ; fragment validé.
    #[test]
    fn encodage_url() {
        assert!(guide_references_managed("[t](a%20b.md)", &set(&["a b.md"])));
        let p = set(&["x.md"]);
        assert!(guide_references_managed("[t](x%2Emd#section)", &p));
        // fragment à échappement invalide ⇒ Parse échoue ⇒ match ignoré ;
        // la boucle floue ne rattrape pas (« x.md » absent en clair de la ligne)
        assert!(!guide_references_managed("[t](x%2Emd#%zz)", &p));
    }

    /// Boucle floue : entrée de paths en clair ou "](Base(entrée))".
    #[test]
    fn retrait_par_chemin_conserve_dans_paths() {
        let paths = set(&["docs/guide.md"]);
        assert!(guide_references_managed(
            "cf. docs/guide.md pour le détail",
            &paths
        ));
        assert!(guide_references_managed("texte ](guide.md) suite", &paths));
        assert!(!guide_references_managed("texte sans référence", &paths));
    }

    /// Faux positif de texte : jamais signalé.
    #[test]
    fn faux_positif_de_texte() {
        let paths = set(&["guide.md"]);
        assert!(!guide_references_managed(
            "une [note] (entre parenthèses) ici",
            &paths
        ));
        assert!(!guide_references_managed("[note](autre-chose.md)", &paths));
        assert!(!guide_references_managed(
            "citation ](sans-crochet-ouvrant)",
            &paths
        ));
        assert!(!guide_references_managed(
            "[site](https://exemple.org/page) et [doc](http://exemple.org/a)",
            &paths
        ));
    }

    /// Chemin nettoyé puis joint à .claude/field-guide.
    #[test]
    fn repertoire_field_guide_et_nettoyage() {
        assert!(guide_references_managed(
            "[g](./x/../a.md)",
            &set(&[".claude/field-guide/a.md"])
        ));
        assert!(guide_references_managed(
            "[g](../a.md)",
            &set(&[".claude/a.md"])
        ));
    }

    /// Cas limites net/url : trims, cible vide, erreurs de Parse, '*'.
    #[test]
    fn cas_limites_net_url() {
        assert!(guide_references_managed(
            "[a]( <g.md> )",
            &set(&[".claude/field-guide/g.md"])
        ));
        assert!(guide_references_managed("[a]( )", &set(&["."]))); // Parse("") → Path ""
        let vide: BTreeSet<String> = BTreeSet::new();
        assert!(!guide_references_managed("[a](:x.md)", &vide)); // erreur Parse
        assert!(!guide_references_managed("[a](1a:b.md)", &vide)); // ':' 1er segment
        assert!(guide_references_managed("[a](*)", &set(&["*"])));
    }
}

#[cfg(test)]
mod percent_userinfo_regression {
    /// A valid empty-host URL is a witness; malformed userinfo must not turn a
    /// Go non-reference into a managed link. Inputs are inert protocol fixtures.
    #[test]
    fn invalid_userinfo_is_not_a_managed_reference() {
        let paths = [".claude/field-guide/g.md".to_owned()]
            .into_iter()
            .collect();
        assert!(super::guide_references_managed(
            "[x](//%20@/%67.md)",
            &paths
        ));
        assert!(!super::guide_references_managed(
            "[x](//%zz@/%67.md)",
            &paths
        ));
    }
}
