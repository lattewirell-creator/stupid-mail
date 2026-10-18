const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const values=new Map,els=new Map;class E{constructor(){this.value='';this.innerHTML='';this.dataset={};this.style={setProperty(){}};this.classList={toggle(){}}}focus(){}showModal(){this.open=true}}
const get=s=>{if(!els.has(s))els.set(s,new E);return els.get(s)};
const context={console,crypto:require('node:crypto'),Date,Set,Object,escape:s=>String(s).replace(/[<>&"]/g,c=>({'<':'&lt;','>':'&gt;','&':'&amp;','"':'&quot;'}[c])),document:{documentElement:new E,querySelector:s=>s==='dialog[open]'?null:get(s)},$:get,localStorage:{getItem:k=>values.get(k)||null,setItem:(k,v)=>values.set(k,v)},setInterval(){},addEventListener(){},toast(){}};context.window=context;
vm.runInNewContext(fs.readFileSync('dist/customize.js','utf8'),context);
get('#look-theme').onchange({target:{value:'dark'}});assert.equal(context.document.documentElement.dataset.theme,'dark');assert.equal(JSON.parse(values.get('stupid-mail-customize')).theme,'dark');
get('#widget-task-input').value='<script>do not execute</script>';get('#widget-task-form').onsubmit({preventDefault(){}});assert.ok(get('#widgets').innerHTML.includes('&lt;script&gt;'));assert.equal(JSON.parse(values.get('stupid-mail-checklist')).length,1);
get('#widget-options').onchange({target:{dataset:{widget:'checklist'},checked:false}});assert.ok(!JSON.parse(values.get('stupid-mail-customize')).widgets.includes('checklist'));assert.equal(JSON.parse(values.get('stupid-mail-checklist')).length,1);
get('#look-reset').onclick();assert.equal(JSON.parse(values.get('stupid-mail-checklist')).length,1);assert.ok(JSON.parse(values.get('stupid-mail-customize')).widgets.includes('checklist'));
get('#widget-options').onchange({target:{dataset:{widget:'clock'},checked:true}});assert.ok(JSON.parse(values.get('stupid-mail-customize')).widgets.includes('clock'));
console.log('Passed appearance persistence, safe task text, widget removal/addition, and reset without task loss.');
