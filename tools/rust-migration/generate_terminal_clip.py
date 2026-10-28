#!/usr/bin/env python3
"""Regenerate fixed Unicode15 tables from an explicit cached uniseg0.4.7 source.

No network or Go invocation. Validate complete source fingerprints and every table
row before atomic publication; --check compares without writing. Source pinning
is a migration contract, not an operator policy. Never execute source expressions.
"""
import argparse
import hashlib
import os
from pathlib import Path
import re
import tempfile

# Exact observed upstream files, added by the migration owner below.
EXPECTED = {'properties.go': 'ff66c81730c902cac9d8d7bfaa4a1d4e914ef5f9a04f9b525aad4e1cb507b94f', 'graphemeproperties.go': '321483625d9a4aa92e4a5730df51c59d5b49725cd20e3879749488e5628b3da8', 'eastasianwidth.go': '1cd2368ddedc56182d2f64576f2a713a2ad6234265d940575285f61006527c5b', 'emojipresentation.go': 'c4e1921aafedd787a29a0a6e329b85c6833eb1cfad0cfaf00450783f99e33b9c', 'LICENSE.txt': 'a59885f5f0f3b3c07cf9444db5fe399b6f0791eac82055f7b85cc65500551039'}
TABLES = {'graphemeproperties.go': ('graphemeCodePoints','GRAPHEME'),
          'eastasianwidth.go': ('eastAsianWidth','EAST_ASIAN'),
          'emojipresentation.go': ('emojiPresentation','EMOJI')}

def constants(text):
    """Decode the supported Go const block; iota is declaration ordinal, not value."""
    block = re.search(r'const\s*\((.*?)\n\)', text, re.S)
    if block is None: raise ValueError('missing property constants')
    result={}; ordinal=0; previous=None
    for line in block.group(1).splitlines():
        line=line.split('//',1)[0].strip()
        if not line: continue
        match=re.fullmatch(r'(pr\w+)(?:\s*=\s*(0|iota))?',line)
        if match is None: raise ValueError('unsupported property declaration')
        name,expression=match.groups(); expression=expression or previous
        if expression is None or name in result: raise ValueError('invalid property declaration')
        result[name]=ordinal if expression=='iota' else 0
        previous=expression; ordinal+=1
    if result.get('prXX')!=0 or result.get('prAny')!=1: raise ValueError('unexpected iota identity')
    return result

def table(text,name,props):
    """Parse every pinned row and reject gaps in parsing, overlaps or invalid code-point bounds."""
    opening='var '+name+' = [][3]int{'
    if text.count(opening)!=1: raise ValueError('table declaration mismatch')
    body,tail=text.split(opening,1)[1].split('\n}',1)
    if tail.strip(): raise ValueError('unsupported table tail')
    rows=[]; last=-1
    for line in body.splitlines():
        line=line.split('//',1)[0].strip()
        if not line: continue
        m=re.fullmatch(r'\{\s*(0x[0-9A-Fa-f]+),\s*(0x[0-9A-Fa-f]+),\s*(pr\w+)\},',line)
        if m is None: raise ValueError('unsupported table row')
        low,high,symbol=m.groups();low=int(low,16);high=int(high,16)
        if not 0<=low<=high<=0x10ffff or low<=last or symbol not in props:
            raise ValueError('invalid or overlapping table range')
        rows.append((low,high,symbol));last=high
    if not rows:raise ValueError('empty table')
    return rows

def rust_name(symbol):
    """Map agreed property identifiers to Rust uppercase names without changing their IDs."""
    return 'PR_'+re.sub(r'([a-z0-9])([A-Z])',r'\1_\2',symbol[2:]).upper()

def generate(source):
    """Validate pinned sources and render deterministic tables plus exact Go ASCII paths."""
    data={}
    for name,expected in EXPECTED.items():
        raw=(source/name).read_bytes()
        if hashlib.sha256(raw).hexdigest()!=expected:raise ValueError('source fingerprint mismatch: '+name)
        data[name]=raw.decode('utf-8')
    props=constants(data['properties.go']); parsed={}
    for name,(go,rust) in TABLES.items():parsed[rust]=table(data[name],go,props)
    used={s for rows in parsed.values() for _,_,s in rows}|{'prAny','prCR','prLF','prControl','prNa','prN'}
    header='//! Generated from uniseg0.4.7 Unicode15; DO NOT EDIT.\n//! Copyright (c) 2019 Oliver Kuederle, MIT; see UNISEG-LICENSE.txt.\n//! Unicode data source: <https://www.unicode.org/Public/15.0.0/ucd/>\n//! Unicode terms: <https://www.unicode.org/license.html>\n'
    header+=''.join('//! '+n+' sha256='+h+'\n' for n,h in EXPECTED.items())
    out=[header]
    for symbol,value in props.items():
        if symbol in used:out.append(f'pub(super) const {rust_name(symbol)}:i32={value};\n')
    for name,rows in parsed.items():
        out.append(f'static {name}:&[(u32,u32,i32)]=&[\n')
        out.extend(f'({low:#x},{high:#x},{rust_name(symbol)}),\n' for low,high,symbol in rows)
        out.append('];\n')
    out.append('''/// Lookup in disjoint sorted protocol ranges; absent property is Go zero.
fn lookup(table:&[(u32,u32,i32)],r:u32)->i32 {
    match table.binary_search_by(|&(low,high,_)| {
        if r<low {std::cmp::Ordering::Greater} else if r>high {std::cmp::Ordering::Less} else {std::cmp::Ordering::Equal}
    }) {Ok(i)=>table[i].2,Err(_)=>0}
}
/// Go grapheme property, including exact ASCII fastpaths; pure, no policy.
pub(super) fn grapheme_property(r:u32)->i32 {
    if (0x20..=0x7e).contains(&r) {return PR_ANY;}
    if r==10 {return PR_LF;}
    if r==13 {return PR_CR;}
    if r<=31 || r==127 {return PR_CONTROL;} lookup(GRAPHEME,r)
}
/// Go East Asian Width property; pure lookup, not terminal/font detection.
pub(super) fn east_asian_property(r:u32)->i32 {
    if (0x20..=0x7e).contains(&r) {return PR_NA;}
    if r<=31 || r==127 {return PR_N;} lookup(EAST_ASIAN,r)
}
/// Go emoji presentation property; pure table lookup.
pub(super) fn emoji_presentation(r:u32)->i32 {lookup(EMOJI,r)}
''')
    return ''.join(out).encode()

def publish(path,data):
    """Write/fsync beside the destination, then replace; pre-rename failure preserves original.

    Retain permissions; post-rename directory-sync failure means visible uncertain
    durability, never a rollback claim. --check performs none of these effects.
    """
    fd,name=tempfile.mkstemp(dir=path.parent,prefix='.unicode-')
    try:
        try:
            stream=os.fdopen(fd,'wb')
        except BaseException as primary:
            try:os.close(fd)
            except BaseException as cleanup:
                raise BaseExceptionGroup('publication stream and descriptor cleanup',[primary,cleanup])
            raise
        with stream:
            os.fchmod(stream.fileno(),path.stat().st_mode&0o777 if path.exists() else 0o644)
            stream.write(data);stream.flush();os.fsync(stream.fileno())
        os.replace(name,path)
        directory=os.open(path.parent,os.O_RDONLY|os.O_DIRECTORY)
        try:os.fsync(directory)
        finally:os.close(directory)
    finally:
        if os.path.exists(name):os.unlink(name)

def main():
    """Require explicit paths; propagate validation/IO failures and reject stale generated bytes."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--check',action='store_true');args=parser.parse_args()
    data=generate(args.source)
    if args.check:
        if args.output.read_bytes()!=data:raise ValueError('generated terminal tables differ')
        print('PASS pinned terminal tables byte-identical')
    else:publish(args.output,data);print('Generated pinned terminal tables:',len(data),'bytes')
if __name__=='__main__':main()
