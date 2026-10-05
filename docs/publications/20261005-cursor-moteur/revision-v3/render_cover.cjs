const fs=require('fs');
const {chromium}=require('/home/fpizzi/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
(async()=>{const browser=await chromium.launch({executablePath:'/opt/google/chrome/chrome',args:['--no-sandbox']});try{
const page=await browser.newPage({viewport:{width:1920,height:1080},deviceScaleFactor:1});
await page.setContent('<body style="margin:0">'+fs.readFileSync(__dirname+'/cover.svg','utf8')+'</body>');
await page.locator('svg').screenshot({path:__dirname+'/couverture_linkedin_1920x1080.png'});
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exit(1)});
