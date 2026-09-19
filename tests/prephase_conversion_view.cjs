'use strict';
// Read-only visual recipe against an isolated preparation with created missions.
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),puppeteer=require('puppeteer');
const url=process.argv[2],out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
(async()=>{const browser=await puppeteer.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});try{
 const page=await browser.newPage(),errors=[],themes=[];page.on('pageerror',e=>errors.push(e.message));await page.setViewport({width:1440,height:1000});await page.goto(url);await page.waitForSelector('#conversion-state:not([hidden])');
 for(const theme of ['etat','sombre']){
  if(await page.$eval('html',e=>e.dataset.theme)!==theme)await page.click('#theme');await page.click('#conversion-open');await page.waitForSelector('#conversion-dialog[open]');themes.push(await require('./prephase_theme.cjs')(page));await page.screenshot({path:path.join(out,'release-'+theme+'.png'),fullPage:true});
  await page.setViewport({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  const bounds=await page.$eval('#conversion-confirm',e=>{const r=e.getBoundingClientRect();return {top:r.top,bottom:r.bottom,height:innerHeight}});assert.ok(bounds.top>=0&&bounds.bottom<=bounds.height,JSON.stringify(bounds));
  await page.screenshot({path:path.join(out,'release-mobile-'+theme+'.png'),fullPage:true});await page.keyboard.press('Escape');assert.equal(await page.evaluate(()=>document.activeElement.id),'conversion-open');await page.setViewport({width:1440,height:1000});
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'themes.json'),JSON.stringify({status:'PASS',themes,checks:['Autorisation visible sur mobile sans défiler','Défilement limité au contenu de revue','Échap rend le focus','Aucune erreur JS'],errors},null,2));console.log('PASS: conversion modal, both themes, mobile actions visible, focus restored');
}finally{await browser.close()}})().catch(e=>{console.error(e);process.exitCode=1});
