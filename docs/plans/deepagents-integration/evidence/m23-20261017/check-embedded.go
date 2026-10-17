// Resource-only compilation probe. Does not import the engine or open a store.
package main

import (
 "bytes"
 "crypto/sha256"
 "encoding/hex"
 "encoding/json"
 "fmt"
 "os"
 "path/filepath"

 resources "swarm.local/companion"
)

func main() {
 expected, err := os.ReadFile(os.Args[1]); must(err)
 var hashes map[string]string
 must(json.Unmarshal(expected, &hashes))
 for name, digest := range hashes {
  content, e := resources.Web.ReadFile("web/"+name); must(e)
  actual := sha256.Sum256(content)
  if hex.EncodeToString(actual[:]) != digest { panic("embedded candidate mismatch: "+name) }
  source, e := os.ReadFile(filepath.Join("web",name)); must(e)
  if !bytes.Equal(source,content) { panic("source changed during resource probe: "+name) }
 }
 page, err := resources.Web.ReadFile("web/project-memory.html"); must(err)
 for _, dependency := range []string{`href="/signals.css"`,`src="/signals.js"`,`src="/project-memory.js"`} {
  if !bytes.Contains(page,[]byte(dependency)) { panic("missing page dependency: "+dependency) }
 }
 out, err := json.Marshal(map[string]any{"status":"PASS","embedded_assets":hashes,"engine_compiled":false,"store_opened":false}); must(err)
 fmt.Println(string(out))
}

func must(err error) { if err != nil { panic(err) } }
