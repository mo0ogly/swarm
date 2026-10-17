//go:build linux

package engine

import (
    "encoding/json"
    "os"
    "path/filepath"
    "strings"
    "testing"
)

func TestMemoryAuditActualProviderReceivesSelectedSources(t *testing.T) {
    for _,explicit:=range []bool{false,true} {
        name:="implicit-index"
        if explicit {name="explicit-lesson-profile"}
        t.Run(name,func(t *testing.T){
            s:=storeTest(t)
            w,launch:=setupAgent(t,s)
            dir,err:=generatedDir(s.root,".claude/field-guide")
            if err!=nil {t.Fatal(err)}
            if err=os.WriteFile(filepath.Join(dir,"index.md"),[]byte("# INDEX_DELIVERY_AUDIT\n\n- [Leçon](lesson.md)\n"),0600);err!=nil {t.Fatal(err)}
            if err=os.WriteFile(filepath.Join(dir,"lesson.md"),[]byte("# LESSON_BODY_DELIVERY_AUDIT\n"),0600);err!=nil {t.Fatal(err)}
            if explicit {
                profile:=profileFixture(t,s)
                prior,_,err:=s.projectContext("worker")
                if err!=nil {t.Fatal(err)}
                profile.Instructions=append(profile.Instructions,ProjectInstruction{Path:".claude/field-guide/lesson.md",Roles:[]string{"worker"}})
                if err=s.applyProjectProfile(profile,prior.ProfileSHA256);err!=nil {t.Fatal(err)}
            }
            script:=filepath.Join(t.TempDir(),"capture-fixture")
            capture:=filepath.Join(t.TempDir(),"received-prompt.txt")
            if err=os.WriteFile(script,[]byte("#!/bin/sh\ncat >\"$1\"\n"),0700);err!=nil {t.Fatal(err)}
            providers:=Providers{Schema:1,Providers:map[string]Provider{"fixture":{Command:script,Args:[]string{capture}}}}
            raw,err:=json.Marshal(providers)
            if err!=nil {t.Fatal(err)}
            if err=os.WriteFile(filepath.Join(s.root,".swarm/providers.json"),raw,0600);err!=nil {t.Fatal(err)}
            a,_,err:=s.prepare(w.ID,launch)
            if err!=nil {t.Fatal(err)}
            if err=s.supervise(a.ID);err!=nil {t.Fatal(err)}
            received,err:=os.ReadFile(capture)
            if err!=nil {t.Fatal(err)}
            if !strings.Contains(string(received),"INDEX_DELIVERY_AUDIT") {t.Fatal("index missing from actual stdin")}
            if strings.Contains(string(received),"LESSON_BODY_DELIVERY_AUDIT")!=explicit {t.Fatal("actual lesson inclusion did not follow explicit project profile")}
            t.Logf("OBSERVED: actual fixture provider received index; lesson_body_received=%v; no LLM and no acceptance",explicit)
        })
    }
}
