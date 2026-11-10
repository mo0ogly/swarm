#!/usr/bin/env python3
"""Bind current generated controller docs to exact sources and purpose prose.
Local source/doc check only, without independent approval or runtime inference.
Reuse the maintained HTML parser; do not infer freshness from Cargo log caching.
"""
from pathlib import Path
import importlib.util
ROOT=Path(__file__).resolve().parents[1]
SPEC=importlib.util.spec_from_file_location("paea_doc_source",ROOT/"tests/paea_initial_rustdoc_test.py")
BASE=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)
BASE.SOURCES=[
 ("swarm_linux_identity_channel","lib.rs","rust/crates/swarm-linux-identity-channel/src/lib.rs"),
 ("swarm_enrollment_authority","personal_service.rs","rust/crates/swarm-enrollment-authority/src/personal_service.rs"),
 ("swarm_enrollment_authority","personal_service/controller_packet.rs","rust/crates/swarm-enrollment-authority/src/personal_service/controller_packet.rs")]
BASE.PAGES=[
 ("swarm_linux_identity_channel/struct.Namespaces.html","Compare exact mount and PID namespace objects"),
 ("swarm_enrollment_authority/personal_service/struct.Service.html","Check an actual kernel packet"),
 ("swarm_enrollment_authority/personal_service/controller_packet/struct.Error.html","All immediate failure branches"),
 ("swarm_enrollment_authority/personal_service/controller_packet/fn.run.html","Sole production caller passes its protected startup UID")]
def main():
 """Require all three exact source pages and four function contracts; read only."""
 BASE.main()
 print("CONTROLLER_SOURCE_BOUND_RUSTDOC_PASS sources=3 pages=4",flush=True)
if __name__=="__main__":main()
