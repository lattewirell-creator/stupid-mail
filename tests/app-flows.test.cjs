// Exercise real app handlers with a small DOM/storage fixture. No real inbox or browser account.
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),crypto=require('node:crypto');
const html=fs.readFileSync('dist/index.html','utf8');
class Element{
 constructor(){this.style={};this.value='';this.textContent='';this.innerHTML='';this.hidden=false;this.open=false;this.disabled=false;this.dataset={};this.classList={toggle(){},add(){},remove(){}};this.listeners={};this.children=[];}
 addEventListener(n,f){(this.listeners[n]??=[]).push(f)}showModal(){this.open=true}close(){this.open=false}focus(){}setAttribute(){}append(x){this.children.push(x)}replaceChildren(){this.children=[]}getBoundingClientRect(){return{x:0,y:0,width:95,height:95}}
}
const elements=new Map([...html.matchAll(/id="([^"]+)"/g)].map(m=>['#'+m[1],new Element()]));for(const name of ['.filters','.buddy-caption'])elements.set(name,new Element());
const store=new Map();let failSave=false,sendCount=0;
const eventListeners={};const document={activeElement:{tagName:'BODY'},querySelector(s){if(s==='dialog[open]')return [...elements.values()].find(x=>x.open);assert.ok(elements.has(s),'Known selector '+s);return elements.get(s)},querySelectorAll(){return []},createElement(){return new Element()},addEventListener(){}};
const context=vm.createContext({document,console,crypto,Date,Map,Set,Promise,structuredClone,Event,localStorage:{getItem:k=>store.get(k)||null,setItem:(k,v)=>{if(failSave)throw Error('Full disk');store.set(k,v)}},setTimeout:()=>1,clearTimeout(){},setInterval(){},matchMedia:()=>({matches:true}),requestAnimationFrame:()=>1,innerWidth:1000,innerHeight:800,sendFixture:async()=>{sendCount++;return{messageID:'<test@icloud.com>'}}});
context.window=context;context.addEventListener=(n,f)=>(eventListeners[n]??=[]).push(f);context.dispatchEvent=e=>{for(const f of eventListeners[e.type]||[])f(e)};
for(const file of ['mail-logic.js','app.js'])vm.runInContext(fs.readFileSync('dist/'+file,'utf8'),context);
const run=s=>vm.runInContext(s,context);
(async()=>{
 run("openMessage(2)");const original=run('current.id');run("document.querySelector('#snooze-message').onclick()");assert.equal(run('MailLogic.snoozed(mails.find(m=>m.id===2))'),true);assert.equal(run("folderMail('quiet').some(m=>m.id===2)"),false);
 run('undoChange()');assert.equal(run('MailLogic.snoozed(mails.find(m=>m.id===2))'),false);
 run("openMessage(2);document.querySelector('#mark-unread').onclick()");assert.equal(run('mails.find(m=>m.id===2).unread'),true);
 run("openMessage(2);document.querySelector('#trash').onclick();openMessage(2);document.querySelector('#restore-message').onclick()");assert.equal(run('mails.find(m=>m.id===2).folder'),'inbox');
 run("account='fixture@icloud.com';bridge=async(action)=>action==='send'?sendFixture():{};compose();document.querySelector('#to').value='test@example.com';document.querySelector('#subject').value='Test';document.querySelector('#body').value='Unsent writing'");
 failSave=true;await run("document.querySelector('#compose-form').onsubmit({preventDefault(){}})");assert.equal(sendCount,0);assert.equal(elements.get('#composer').open,true);assert.equal(elements.get('#body').value,'Unsent writing');assert.equal(run('sending'),false);
 failSave=false;await run("document.querySelector('#compose-form').onsubmit({preventDefault(){}})");assert.equal(sendCount,1);assert.equal(elements.get('#composer').open,false);assert.equal(run("mails.filter(m=>m.folder==='sent'&&m.subject==='Test').length"),1);
 run("mails=[{id:'remote',from:'Jamie',email:'jamie@example.com',body:'Before',subject:'Test',time:'2026-10-17',remote:true,folder:'inbox'}];current=mails[0]");
 await run("applySync({email:account,messages:[{id:'remote',from:'Jamie',email:'jamie@example.com',body:'After',subject:'Test',time:'2026-10-17'}]})");assert.equal(run('current===mails[0]'),true);assert.equal(run('current.body'),'After');
 run("window.useBuddyDraft('General suggestion',null)");assert.equal(elements.get('#to').value,'');assert.equal(elements.get('#body').value,'General suggestion');
 assert.equal(await run('window.flushDraft()'),true);
 console.log('Passed snooze/undo, restore, unread, failed-save send blocking, send completion, refresh identity, and Buddy draft-context flows.');
})().catch(e=>{console.error(e);process.exitCode=1});
