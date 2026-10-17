//go:build linux

package engine

import (
    "os"
    "path/filepath"
    "strings"
    "testing"
)

func TestMemoryAuditRetexPublicationReachesLaunchPrompt(t *testing.T) {
    s := storeTest(t)
    w, launch := setupAgent(t, s)
    w = memorySeed(t, s, w)
    r := Retex{ID:"r-memory-audit",Source:dialogueHits(w)[0].Ref,Problem:"Audit mémoire",Nature:"fait",Proposal:"Retenir une leçon",Effort:"faible",Risk:"Perte de contexte",Criteria:"Source reçue",Proof:"proof.txt"}
    w = retexTestAction(t,s,w,"retex-save",r,"")
    w = retexTestAction(t,s,w,"retex-task",r,"")
    id := w.Retex[0].Task
    raw := []byte(strings.ReplaceAll(string(fixture(t,s.root)),`"t1"`,`"`+id+`"`))
    ev, err := evaluate(raw,s.root,"delivery")
    if err != nil { t.Fatal(err) }
    w, err = s.mutate(w.ID,"fixture-gate",newID("e-"),w.Revision,[]byte(`{}`),func(w *Work) error {
        task,_ := w.task(id)
        task.Status="accepted"
        task.Gate=&GateRecord{Document:raw,Evaluation:ev,At:now()}
        return nil
    })
    if err != nil { t.Fatal(err) }
    w = retexTestAction(t,s,w,"retex-status",r,"verifie")
    q := webRequest{Kind:"retex-guide-preview",Work:w.ID,Task:r.ID,Event:newID("e-"),Revision:w.Revision,Note:"UNIQUE_MEMORY_LESSON_BODY_AUDIT"}
    v, err := s.retexAction(q)
    if err != nil { t.Fatal(err) }
    q.ContextHash=v.(map[string]any)["digest"].(string)
    q.Kind="retex-guide"
    if _,err=s.retexAction(q);err!=nil {t.Fatal(err)}
    w,err=s.get(w.ID)
    if err!=nil {t.Fatal(err)}
    launch.Revision=w.Revision
    a,created,err:=s.prepareLaunch(w.ID,launch,true)
    if err!=nil || created {t.Fatalf("preview: %v; created=%v",err,created)}
    if !strings.Contains(a.Prompt,"retex-r-memory-audit.md") {t.Fatal("published guide index did not reach prepared prompt")}
    if strings.Contains(a.Prompt,"UNIQUE_MEMORY_LESSON_BODY_AUDIT") {t.Fatal("audit assumption contradicted: lesson body was loaded automatically")}
    t.Log("OBSERVED: verified RETEX publication reached launch prompt as index link; lesson body was not injected")
}

func TestMemoryAuditOversizedImplicitGuideIsOmitted(t *testing.T) {
    s:=storeTest(t)
    w,launch:=setupAgent(t,s)
    dir,err:=generatedDir(s.root,".claude/field-guide")
    if err!=nil {t.Fatal(err)}
    if err=os.WriteFile(filepath.Join(dir,"index.md"),[]byte("OVERSIZED_GUIDE_MARKER"+strings.Repeat("x",16000)),0600);err!=nil {t.Fatal(err)}
    a,created,err:=s.prepareLaunch(w.ID,launch,true)
    if err!=nil || created {t.Fatalf("preview refused: %v; created=%v",err,created)}
    if strings.Contains(a.Prompt,"OVERSIZED_GUIDE_MARKER") {t.Fatal("oversized guide unexpectedly injected")}
    t.Log("OBSERVED: oversized guide outside an explicit project profile was omitted while preview still succeeded")
}
