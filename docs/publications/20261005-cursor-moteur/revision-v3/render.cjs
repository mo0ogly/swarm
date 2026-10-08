const fs=require('fs');
const {chromium}=require('/home/fpizzi/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
(async()=>{const browser=await chromium.launch({executablePath:'/opt/google/chrome/chrome',args:['--no-sandbox']});try{
const page=await browser.newPage({viewport:{width:1800,height:2400},deviceScaleFactor:2});
await page.setContent('<body style="margin:0;background:white"><div id="diagram"></div></body>');
await page.addScriptTag({path:'/home/fpizzi/wattson_devcontainer/flaskProject/static/js/mermaid.min.js'});
const svg=await page.evaluate(async s=>{mermaid.initialize({startOnLoad:false,securityLevel:'strict',theme:'base',themeVariables:{fontFamily:'Arial',fontSize:'21px',lineColor:'#496781'},flowchart:{useMaxWidth:false,htmlLabels:true,nodeSpacing:30,rankSpacing:35}});const r=await mermaid.render('architecture',s);document.querySelector('#diagram').innerHTML=r.svg;return r.svg;},fs.readFileSync(__dirname+'/orchestration.mmd','utf8'));
fs.writeFileSync(__dirname+'/orchestration.svg',svg);await page.locator('#diagram svg').screenshot({path:__dirname+'/orchestration.png'});
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exit(1)});
