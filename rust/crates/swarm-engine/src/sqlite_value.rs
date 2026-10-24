//! database/sql scalar scan compatibility, not business validation.
use rusqlite::types::ValueRef;
// database/sql/driver.Bool: floats and NULL are not valid booleans.
pub(crate) fn boolean(value: ValueRef<'_>, column: usize) -> rusqlite::Result<bool> {
    match value {
        ValueRef::Integer(0) => Ok(false),
        ValueRef::Integer(1) => Ok(true),
        ValueRef::Text(b"1" | b"t" | b"T" | b"TRUE" | b"true" | b"True")
        | ValueRef::Blob(b"1" | b"t" | b"T" | b"TRUE" | b"true" | b"True") => Ok(true),
        ValueRef::Text(b"0" | b"f" | b"F" | b"FALSE" | b"false" | b"False")
        | ValueRef::Blob(b"0" | b"f" | b"F" | b"FALSE" | b"false" | b"False") => Ok(false),
        _ => Err(rusqlite::Error::InvalidColumnType(
            column,
            "boolean".into(),
            value.data_type(),
        )),
    }
}
pub(crate) fn integer(value: ValueRef<'_>, column: usize) -> rusqlite::Result<i64> {
    match value {
        ValueRef::Integer(v) => Ok(v),
        ValueRef::Blob(v) | ValueRef::Text(v) => {
            let text = std::str::from_utf8(v).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(column, value.data_type(), Box::new(e))
            })?;
            text.parse().map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(column, value.data_type(), Box::new(e))
            })
        }
        ValueRef::Real(v) if v.is_finite() && v.fract() == 0.0 && v.abs() < 1_000_000.0 => {
            Ok(v as i64)
        }
        _ => Err(rusqlite::Error::InvalidColumnType(
            column,
            "integer".into(),
            value.data_type(),
        )),
    }
}
// strconv.FormatFloat('g', -1, 64) for database/sql string destinations.
fn float_text(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value == f64::INFINITY {
        return "+Inf".into();
    }
    if value == f64::NEG_INFINITY {
        return "-Inf".into();
    }
    let text = value.to_string();
    let magnitude = value.abs();
    if magnitude == 0.0 || (1e-4..1e6).contains(&magnitude) {
        return text;
    }
    let (sign, body) = text
        .strip_prefix('-')
        .map_or(("", text.as_str()), |v| ("-", v));
    let point = body.find('.').unwrap_or(body.len());
    let digits = body.replace('.', "");
    let first = digits.find(|c| c != '0').expect("nonzero finite float");
    let significant = digits[first..].trim_end_matches('0');
    let exponent = point as i64 - first as i64 - 1;
    let fraction = if significant.len() > 1 {
        format!(".{}", &significant[1..])
    } else {
        String::new()
    };
    format!("{sign}{}{fraction}e{exponent:+03}", &significant[..1])
}
pub(crate) fn string(value: ValueRef<'_>, column: usize) -> rusqlite::Result<Vec<u8>> {
    match value {
        ValueRef::Text(v) | ValueRef::Blob(v) => Ok(v.to_vec()),
        ValueRef::Integer(v) => Ok(v.to_string().into_bytes()),
        ValueRef::Real(v) => Ok(float_text(v).into_bytes()),
        _ => Err(rusqlite::Error::InvalidColumnType(
            column,
            "string".into(),
            value.data_type(),
        )),
    }
}
