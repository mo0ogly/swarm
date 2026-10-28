#!/usr/bin/env python3
"""Bind exchange Rustdoc to exact current sources and visible API contracts.
Reuses the maintained source HTML parser. Local documentation proof only.
"""
import paea_packet_response_rustdoc_test as check
check.SOURCES=[('swarm_enrollment_authority','personal_service.rs','rust/crates/swarm-enrollment-authority/src/personal_service.rs'),('swarm_enrollment_authority','personal_service/controller_packet.rs','rust/crates/swarm-enrollment-authority/src/personal_service/controller_packet.rs'),('swarm_enrollment_authority','personal_service/exchange.rs','rust/crates/swarm-enrollment-authority/src/personal_service/exchange.rs')]
check.PAGES=[('swarm_enrollment_authority/personal_service/struct.Service.html','Receive one guarded packet, apply the unchanged strict atomic controller'),('swarm_enrollment_authority/personal_service/struct.Reply.html','Borrow every independent business failure'),('swarm_enrollment_authority/personal_service/exchange/struct.Error.html','Retain first refusal plus an independent packet-close failure'),('swarm_enrollment_authority/personal_service/exchange/enum.Cause.html','First refusal and its original owned data.')]
if __name__=='__main__':check.main()
