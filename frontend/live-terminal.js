// Read-only xterm renderer for the live harness window (web/live-session.js).
// Exposes globalThis.SwarmLiveTerminal; no input is ever relayed: stdin is
// disabled and synthetic or trusted keyboard events never leave the page.
import {Terminal} from '@xterm/xterm';
import {FitAddon} from '@xterm/addon-fit';

/** Map xterm colours to the current theme tokens (re-read on theme change). */
function palette(){
 const css=getComputedStyle(document.documentElement),v=name=>css.getPropertyValue('--wattson-'+name).trim();
 const colors=[v('texte-discret'),v('alerte-encre'),v('succes-encre'),v('attention-encre'),v('lien'),v('info-encre'),v('succes-encre'),v('texte')];
 const t={background:v('champ'),foreground:v('texte'),cursor:v('champ'),cursorAccent:v('champ'),selectionBackground:v('info-fond'),selectionForeground:v('info-encre')};
 ['black','red','green','yellow','blue','magenta','cyan','white'].forEach((key,i)=>{t[key]=colors[i];t['bright'+key[0].toUpperCase()+key.slice(1)]=colors[i]});return t;
}
/** Open a read-only terminal in `host`. Returns {write(text), resize(cols,rows),
 * retheme(), fit()}; `write` takes the recorded PTY text verbatim (ANSI kept). */
function create(host){
 const terminal=new Terminal({cols:96,rows:24,scrollback:5000,fontSize:13,fontFamily:'ui-monospace,monospace',disableStdin:true,cursorBlink:false,screenReaderMode:true,minimumContrastRatio:4.5,theme:palette(),linkHandler:{activate(){},hover(){},leave(){}}});
 const fit=new FitAddon();terminal.loadAddon(fit);terminal.open(host);
 for(const name of ['keydown','keypress','input','paste','compositionstart','compositionupdate','compositionend'])host.addEventListener(name,e=>{e.stopImmediatePropagation()},true);
 terminal.write('\x1b[?25l');// read-only view: no cursor to show
 try{fit.fit()}catch{}
 addEventListener('resize',()=>{try{fit.fit()}catch{}});
 return {write:text=>terminal.write(text),resize(cols,rows){if(cols>0&&rows>0&&(cols!==terminal.cols||rows!==terminal.rows))terminal.resize(cols,Math.max(rows,terminal.rows))},retheme(){terminal.options.theme=palette()},fit(){try{fit.fit()}catch{}},text(){const b=terminal.buffer.active,lines=[];for(let i=0;i<b.length;i++)lines.push(b.getLine(i)?.translateToString(true)??'');return lines.join('\n')}};
}
globalThis.SwarmLiveTerminal={create};
