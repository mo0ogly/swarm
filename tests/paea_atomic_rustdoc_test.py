#!/usr/bin/env python3
"""Verify actual current Rustdoc source bytes and declared function contracts.
Cargo may reuse a fresh page without printing Documenting. Read generated source
HTML, exclude only line-number anchors and compare exact source; then inspect
visible function prose. This is local docs binding, not code/global acceptance.
"""
from pathlib import Path
from html.parser import HTMLParser
import os,hashlib,json
ROOT=Path(__file__).resolve().parents[1]
DOC=Path(os.environ['CARGO_TARGET_DIR'])/'doc'
SOURCES=[('swarm_linux_identity_channel','lib.rs','rust/crates/swarm-linux-identity-channel/src/lib.rs'),('swarm_linux_identity_channel','kernel.rs','rust/crates/swarm-linux-identity-channel/src/kernel.rs'),('swarm_linux_identity_channel','atomic_spawn.rs','rust/crates/swarm-linux-identity-channel/src/atomic_spawn.rs'),('swarm_linux_identity_channel','kernel/atomic_spawn.rs','rust/crates/swarm-linux-identity-channel/src/kernel/atomic_spawn.rs'),('swarm_enrollment_authority','personal_service.rs','rust/crates/swarm-enrollment-authority/src/personal_service.rs'),('swarm_enrollment_authority','personal_service/controller_packet.rs','rust/crates/swarm-enrollment-authority/src/personal_service/controller_packet.rs')]
PAGES=[('swarm_linux_identity_channel/atomic_spawn/fn.launch.html','Create a process with its descriptor atomically'),('swarm_linux_identity_channel/atomic_spawn/struct.Spawned.html','Own the atomically returned process object'),('swarm_linux_identity_channel/atomic_spawn/struct.Failure.html','Native launch failure retains actual invocation')]
class Source(HTMLParser):
    """Only rust source pre/code text, skipping numbered data-nosnippet anchors."""
    def __init__(self):
        """Keep separate rust-pre and line-anchor state; no HTML execution."""
        super().__init__(convert_charrefs=True);self.inside=False;self.skip=False;self.parts=[]
    def handle_starttag(self,tag,attrs):
        """Select actual Rust code pre, not scripts/sidebar/source line numbers."""
        attrs=dict(attrs)
        if tag=='pre'and attrs.get('class')=='rust':self.inside=True
        if self.inside and tag=='a'and'data-nosnippet'in attrs:self.skip=True
    def handle_endtag(self,tag):
        """Close only the numbered anchor/pre; code links remain real code text."""
        if tag=='a':self.skip=False
        if tag=='pre':self.inside=False
    def handle_data(self,value):
        """Retain decoded source characters exactly, without trimming/normalizing."""
        if self.inside and not self.skip:self.parts.append(value)
class Text(HTMLParser):
    """Visible function document text only; used for exact purpose phrases."""
    def __init__(self):
        """No browser/script interaction required for generated function prose."""
        super().__init__(convert_charrefs=True);self.parts=[]
    def handle_data(self,value):
        """Keep render text and normalize whitespace only at comparison."""
        self.parts.append(value)
def main():
    """Require all current source pages and function prose; no cache log heuristic."""
    rows=[]
    for crate,relative,source in SOURCES:
        page=DOC/'src'/crate/(relative+'.html');raw=page.read_bytes();parser=Source();parser.feed(raw.decode());actual=''.join(parser.parts);expected=(ROOT/source).read_text()
        assert actual==expected,'rustdoc-source-byte-mismatch:'+source
        rows.append({'source':source,'source_sha256':hashlib.sha256(expected.encode()).hexdigest(),'html_sha256':hashlib.sha256(raw).hexdigest()})
    pages=[]
    for name,phrase in PAGES:
        raw=(DOC/name).read_bytes();parser=Text();parser.feed(raw.decode());text=' '.join(''.join(parser.parts).split());assert phrase in text,'rustdoc-contract-prose-mismatch:'+name
        pages.append({'path':name,'html_sha256':hashlib.sha256(raw).hexdigest()})
    print(json.dumps({'scope':'current source-bound local docs only','sources':rows,'function_pages':pages}),flush=True)
    print(f'SOURCE_BOUND_RUSTDOC_PASS sources={len(SOURCES)} pages={len(PAGES)}',flush=True)
if __name__=='__main__':main()
