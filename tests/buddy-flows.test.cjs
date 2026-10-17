const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const els=new Map();class E{constructor(){this.hidden=false;this.value='';this.checked=false;this.children=[];this.classList={add(){}}}append(e){this.children.push(e)}replaceChildren(){this.children=[]}focus(){}setAttribute(){}addEventListener(){}}
function $(s){if(!els.has(s))els.set(s,new E());return els.get(s)}
const listeners={};let selected={id:'one',subject:'One',email:'a@example.com',body:'Private one'},resolveAnswer,lastRequest,used;
const context=vm.createContext({$,native:true,console,matchMedia:()=>({matches:true}),localStorage:{getItem:()=>null,setItem(){}},document:{createElement:()=>new E(),querySelectorAll:()=>[],addEventListener(){}},bridge:async(action,fields)=>{if(action==='buddyStatus')return{ready:true,provider:'ollama',model:'fixture'};if(action==='buddyAsk'){lastRequest=fields;return new Promise(resolve=>resolveAnswer=resolve)}},mailForBuddy:()=>selected,useBuddyDraft:(text,id)=>{used={text,id}},addEventListener:(n,f)=>(listeners[n]??=[]).push(f)});context.window=context;
const emit=name=>{for(const f of listeners[name]||[])f()};const tick=()=>new Promise(setImmediate);
vm.runInContext(fs.readFileSync('dist/buddy-ui.js','utf8'),context);
(async()=>{
 $('#buddy-panel').hidden=true;$('#buddy-toggle').onclick();await tick();$('#buddy-include').checked=true;$('#buddy-question').value='Summarize';$('#buddy-form').onsubmit({preventDefault(){}});assert.ok(lastRequest.context.includes('Private one'));
 emit('mail-account-changed');resolveAnswer({answer:'OLD ACCOUNT ANSWER'});await tick();assert.ok(!$('#buddy-log').children.some(x=>x.textContent.includes('OLD ACCOUNT ANSWER')));assert.equal($('#use-buddy-draft').hidden,true);
 $('#buddy-include').checked=true;$('#buddy-question').value='Draft reply';$('#buddy-form').onsubmit({preventDefault(){}});selected={id:'two',subject:'Two',email:'b@example.com',body:'Private two'};emit('mail-selected');resolveAnswer({answer:'Reply for one'});await tick();assert.equal($('#use-buddy-draft').hidden,true);
 $('#buddy-include').checked=false;$('#buddy-question').value='Write a general greeting';$('#buddy-form').onsubmit({preventDefault(){}});assert.equal(lastRequest.context,'');resolveAnswer({answer:'Hello there'});await tick();$('#use-buddy-draft').onclick();assert.deepEqual(used,{text:'Hello there',id:null});
 console.log('Passed Buddy stale-account response, selected-message binding, and no-context draft tests.');
})().catch(e=>{console.error(e);process.exitCode=1});
