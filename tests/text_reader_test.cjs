'use strict';
const assert=require('node:assert/strict'),{parse}=require('../web/text-reader.js');
let count=0;
function check(name,fn){fn();count++;console.log('PASS '+name)}
check('headings, multiline paragraphs and explicit section labels',()=>{
 const blocks=parse('# Objectif\n\nDeux lignes\npour une consigne.\n\nVérifications :\n- test A\n- test B');
 assert.deepEqual(blocks.map(b=>b.type),['heading','paragraph','heading','list']);
 assert.equal(blocks[1].text,'Deux lignes\npour une consigne.');assert.equal(blocks[3].items.length,2);
});
check('ordered lists and written checklists retain their values',()=>{
 const [ordered,checks]=parse('3. Première preuve\n4. Seconde preuve\n\n- [x] Déclaration faite\n- [ ] À vérifier');
 assert.equal(ordered.start,3);assert.deepEqual(checks.items.map(i=>i.checked),[true,false]);
});
check('explicit warning is separated from ordinary quotation',()=>{
 const blocks=parse('> [!WARNING]\n> Conserver les preuves.\n\n> Une citation.');
 assert.equal(blocks[0].kind,'WARNING');assert.equal(blocks[0].text,'Conserver les preuves.');assert.equal(blocks[1].kind,'');
});
check('fenced code is literal, including HTML and instruction-like text',()=>{
 const [code]=parse('```json\n{"key":"<script>bad()</script>"}\n# literal\n```');
 assert.equal(code.type,'code');assert.equal(code.language,'json');assert.match(code.text,/# literal/);
});
check('unterminated fences remain readable code',()=>assert.equal(parse('~~~txt\ntexte sans fin')[0].text,'texte sans fin'));
check('tables preserve escaped pipes and cell content',()=>{
 const [table]=parse('| Contrôle | Résultat |\n| --- | --- |\n| A \\| B | À vérifier |');
 assert.equal(table.type,'table');assert.deepEqual(table.rows,[['A | B','À vérifier']]);
});
check('plain HTML is text and CRLF paragraphs are parsed without losing the source contract',()=>{
 assert.equal(parse('<img src=x onerror=bad()>')[0].text,'<img src=x onerror=bad()>');assert.equal(parse('a\r\nb')[0].text,'a\nb');
});
check('empty and invalid syntax remain explicit',()=>{
 assert.deepEqual(parse(''),[]);assert.equal(parse('| pas un tableau |')[0].type,'paragraph');
 assert.equal(parse('**sans fermeture')[0].text,'**sans fermeture');
});
console.log('PASS '+count+' text reader cases');
