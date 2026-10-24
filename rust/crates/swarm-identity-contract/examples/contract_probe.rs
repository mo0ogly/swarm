//! Test-only differential probe; not part of the Swarm CLI or runtime.
#![forbid(unsafe_code)]
use std::io::{self, BufRead, Write};
use swarm_identity_contract::{digest, name, text, Permission, Principal, Scope};

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) || !value.is_ascii() {
        return Err("invalid test hex".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn check(line: &str) -> Result<bool, String> {
    let fields: Vec<_> = line.split('\t').collect();
    let expected = match fields[0] {
        "name" | "text" | "digest" => 2,
        "principal" => 5,
        "scope" => 4,
        "permission" => 3,
        _ => return Err("unknown test operation".into()),
    };
    if fields.len() != expected {
        return Err("invalid test protocol".into());
    }
    if fields.len() == 2 {
        let value = decode_hex(fields[1])?;
        return Ok(match fields[0] {
            "name" => name(&value),
            "text" => text(&value),
            _ => digest(&value),
        });
    }
    let bytes: Result<Vec<_>, _> = fields[1..]
        .iter()
        .enumerate()
        .map(|(index, field)| {
            if fields[0] == "scope" && index == 1 {
                Ok(field.as_bytes().to_vec())
            } else {
                decode_hex(field)
            }
        })
        .collect();
    let strings: Result<Vec<_>, _> = bytes?.into_iter().map(String::from_utf8).collect();
    let Ok(args) = strings else {
        return Ok(false);
    };
    Ok(match fields[0] {
        "principal" => Principal {
            id: args[0].clone(),
            kind: args[1].clone(),
            issuer: args[2].clone(),
            subject: args[3].clone(),
        }
        .valid(),
        "scope" => Scope {
            work_id: args[0].clone(),
            plan_version: args[1].parse::<i64>().map_err(|e| e.to_string())?,
            plan_digest: args[2].clone(),
        }
        .valid(),
        "permission" => Permission {
            operation: args[0].clone(),
            resource: args[1].clone(),
        }
        .valid(),
        _ => return Err("unknown test operation".into()),
    })
}

fn unicode_table() -> io::Result<()> {
    let mut table = Vec::new();
    for value in (0..=0x10ffff).filter_map(char::from_u32) {
        let value = value.to_string();
        let middle = format!("a{value}b");
        let checks = [
            text(value.as_bytes()),
            text(middle.as_bytes()),
            name(value.as_bytes()),
            name(middle.as_bytes()),
            text(format!("{value}a").as_bytes()),
            text(format!("a{value}").as_bytes()),
        ];
        let bits =
            checks.iter().enumerate().fold(
                0_u8,
                |bits, (i, valid)| {
                    if *valid {
                        bits | (1 << i)
                    } else {
                        bits
                    }
                },
            );
        table.push(bits);
    }
    io::stdout().write_all(&table)
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--unicode"] {
        return unicode_table().map_err(|e| e.to_string());
    }
    if !args.is_empty() {
        return Err("unknown test argument".into());
    }
    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    for line in io::stdin().lock().lines() {
        let valid = check(&line.map_err(|e| e.to_string())?)?;
        writeln!(output, "{}", u8::from(valid)).map_err(|e| e.to_string())?;
    }
    output.flush().map_err(|e| e.to_string())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
