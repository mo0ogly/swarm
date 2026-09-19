"""Static token and DOM contract audit; visual validation is recorded separately."""
import re,json
from pathlib import Path
root=Path('tools/swarm-companion/web')
files=['index.html','cockpit.css','cockpit.js','brainstorm.js','retex.js','cockpit-ux.css','cockpit-help.js','plan.js','plan.css','assistant.js','assistant.css']
text='\n'.join((root/f).read_text() for f in files)
refs=set(re.findall(r'var\((--wattson-[\w-]+)',text))
tokens=(root/'wattson_themes.css').read_text()
missing=sorted(refs-set(re.findall(r'(--wattson-[\w-]+)\s*:',tokens)))
violations=[]
for f in files:
 s=(root/f).read_text()
 for pattern in [r'#[0-9a-fA-F]{3,8}\b',r'\b(?:rgb|rgba|hsl|hsla)\(',r'\b(?:color|background)\s*:\s*(?:white|black)\b',r'\bstyle\s*=',r'\.innerHTML\s*=',r'\b(?:eval|Function)\(']:
  if re.search(pattern,s):violations.append([f,pattern])
ids=re.findall(r'\bid="([^"]+)"',(root/'index.html').read_text())
assert len(ids)==len(set(ids)), 'Duplicate DOM ids'
assert not missing and not violations,(missing,violations)
print(json.dumps({'status':'PASS','tokens_referenced':len(refs),'undefined_in_stylesheet':missing,'violations':violations,'unique_dom_ids':len(ids),'limit':'Computed tokens for each theme and visual hierarchy must also be checked in the browser.'},indent=2))
