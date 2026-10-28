#!/usr/bin/env python3
"""Bind current spawned-process gate docs to exact sources and visible contracts.
Generated source equality and purpose checks are local evidence, not approval of
launch/reaper custody or a distinct-UID daemon. No Cargo log freshness heuristic.
"""
from pathlib import Path
import importlib.util
ROOT=Path(__file__).resolve().parents[1]
SPEC=importlib.util.spec_from_file_location("paea_source_doc",ROOT/"tests/paea_initial_rustdoc_test.py")
BASE=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)
BASE.SOURCES=[
 ("swarm_linux_identity_channel","lib.rs","rust/crates/swarm-linux-identity-channel/src/lib.rs"),
 ("swarm_linux_identity_channel","kernel.rs","rust/crates/swarm-linux-identity-channel/src/kernel.rs"),
 ("swarm_enrollment_authority","personal_service.rs","rust/crates/swarm-enrollment-authority/src/personal_service.rs"),
 ("swarm_enrollment_authority","personal_service/controller_packet.rs","rust/crates/swarm-enrollment-authority/src/personal_service/controller_packet.rs")]
BASE.PAGES=[
 ("swarm_linux_identity_channel/struct.ProcessAnchor.html","Capture a live, caller-owned Child"),
 ("swarm_linux_identity_channel/kernel/fn.child_process.html","Open the caller’s independently owned"),
 ("swarm_linux_identity_channel/kernel/fn.process_identity.html","Require actual pidfs and full64bit inode"),
 ("swarm_linux_identity_channel/kernel/fn.process_live.html","Immediate pidfd termination observation only"),
 ("swarm_enrollment_authority/personal_service/struct.Service.html","Require actual sender binding"),
 ("swarm_enrollment_authority/personal_service/controller_packet/fn.run_spawned.html","Strict additional process binding")]
def main():
 """Require four exact source pages and six purpose contracts; read only."""
 BASE.main()
 print("SPAWN_SOURCE_BOUND_RUSTDOC_PASS sources=4 pages=6",flush=True)
if __name__=="__main__":main()
