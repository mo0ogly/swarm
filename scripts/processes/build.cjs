'use strict';
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'../..'),out=path.resolve(process.argv[2]||path.join(root,'dist/processes'));
if(process.argv.length>3||out===root||out===path.join(root,'web'))throw Error('Usage: node scripts/processes/build.cjs OUTPUT_DIRECTORY');
const files=['processes.html','processes.css','processes.js','wattson_themes.css','swarm-design.css','i18n.js','i18n-en.js','favicon.svg'];
fs.mkdirSync(out,{recursive:true});
const manifest={schema_version:1,entry:'index.html',api:'/api/v1/processes',assets:{}};
for(const name of files){const data=fs.readFileSync(path.join(root,'web',name));fs.writeFileSync(path.join(out,name),data);manifest.assets[name]=crypto.createHash('sha256').update(data).digest('hex')}
fs.copyFileSync(path.join(out,'processes.html'),path.join(out,'index.html'));
fs.writeFileSync(path.join(out,'build-manifest.json'),JSON.stringify(manifest,null,2)+'\n');
console.log(JSON.stringify({output:out,assets:files.length,entry:manifest.entry}));
