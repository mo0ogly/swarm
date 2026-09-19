// Usage: SWARM_TEST_SESSION_URL='http://127.0.0.1:PORT/session/TOKEN?work=WORK' node tests/cockpit_multitab_ui.cjs
const assert=require('node:assert/strict');
const puppeteer=require('puppeteer');
(async()=>{
 const url=process.env.SWARM_TEST_SESSION_URL;
 assert.ok(url,'An isolated authenticated cockpit URL is required');
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 try{
  const pages=[];
  for(let i=0;i<8;i++){
   const page=await browser.newPage();pages.push(page);
   await page.goto(url,{waitUntil:'domcontentloaded',timeout:10000});
   await page.waitForSelector('.graph-responsibility',{timeout:10000});
   assert.ok(await page.$$eval('.graph-responsibility',els=>els.length)>=2);
  }
  await pages[0].bringToFront();
  await pages[0].reload({waitUntil:'domcontentloaded',timeout:10000});
  await pages[0].waitForSelector('.graph-responsibility',{timeout:10000});
  console.log('PASS: eight simultaneous tabs and return to the first cockpit');
 }finally{await browser.close()}
})().catch(e=>{console.error(e);process.exit(1)});
