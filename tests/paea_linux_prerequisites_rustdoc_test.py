#!/usr/bin/env python3
"""Bind standalone observer Rustdoc to current source and API contracts only."""
import paea_packet_response_rustdoc_test as check
check.SOURCES=[('swarm_linux_prerequisites','lib.rs','rust/crates/swarm-linux-prerequisites/src/lib.rs')]
check.PAGES=[('swarm_linux_prerequisites/fn.observe.html',"Observe this process’s Linux identity"),('swarm_linux_prerequisites/struct.Report.html','Version2 native observation;'),('swarm_linux_prerequisites/enum.Observation.html','An unavailable or invalid source is never an observed empty assignment.'),('swarm_linux_prerequisites/struct.Failure.html','Safe diagnostic plus original IO cause.')]
if __name__=='__main__':check.main()
