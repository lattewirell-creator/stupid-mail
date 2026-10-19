(() => {
let settings={ready:false},busy=false,answer='',answerMailID=null,robot=null,epoch=0;
const panel=$('#buddy-panel'),log=$('#buddy-log'),include=$('#buddy-include');
const lookKey='stupid-mail-buddy-browser-look-v1';
let look=BuddyLook.normalize(),preview=null;
// Use the real browser default for migration; retain the old Mail key as a backup.
try{look=BuddyLook.normalize(JSON.parse(localStorage.getItem(lookKey)||'{}'))}catch{}
const titles={hiker:'Hiker',ball:'Ball',eye:'Eye',round:'Round',cube:'Cube',capsule:'Capsule',egg:'Egg',tv:'TV',tophat:'Top hat',lash:'Lashes',dot:'Dots'};
function applyLook(){
 $('#buddy-name').value=look.name;$('#buddy-panel-title').textContent=look.name;$('.buddy-caption').textContent=look.name;
 for(const [key,values]of Object.entries(BuddyLook.choices)){
  const select=$('#buddy-'+key);select.innerHTML=values.map(value=>`<option value="${value}">${titles[value]||value[0].toUpperCase()+value.slice(1)}</option>`).join('')+(/^#/.test(look[key])?`<option value="${look[key]}">Custom colour</option>`:'');select.value=look[key];
  if(['color','glow','outfit'].includes(key))$('#buddy-'+key+'-custom').value=/^#/.test(look[key])?look[key]:(key==='glow'?BuddyLook.glows:BuddyLook.colors)[look[key]];
 }
 $('#buddy-backpack').checked=look.backpack;$('#buddy-scarf').checked=look.scarf;$('#buddy-photo-remove').hidden=!look.image;
 robot?.setLook(look);preview?.setLook(look);
}
function saveLook(patch){look=BuddyLook.normalize({...look,...patch});applyLook();try{localStorage.setItem(lookKey,JSON.stringify(look));$('#buddy-look-status').textContent='Saved on this Mac.'}catch{$('#buddy-look-status').textContent='Could not save Buddy’s look. Free some space and try again.'}}
function bubble(text,who='buddy'){const p=document.createElement('p');p.className=who==='user'?'buddy-question':'buddy-answer';p.textContent=text;log.append(p);log.scrollTop=log.scrollHeight}
function sharing(){const m=window.mailForBuddy();const cloud=settings.provider&&settings.provider!=='ollama';$('#buddy-sharing').textContent=include.checked&&m?`${cloud?'Shares':'Uses locally'} “${m.subject.slice(0,55)}” with ${settings.provider||'your chosen AI'} when you ask.`:'No email is included. Buddy receives only your question.'}
function setStatus(s){settings=s;robot?.setState({sleepy:!s.ready});$('#buddy-provider').textContent=busy?'Thinking…':s.ready?`${s.provider==='ollama'?'Ollama · local':s.provider==='openai'?'OpenAI':'Claude'} · ${s.model}`:'Choose an AI in settings to wake Buddy up.';sharing()}
function open(){panel.hidden=false;preview?.setVisible($('#buddy-customize').open);$('#buddy-toggle').setAttribute('aria-expanded','true');sharing();if(native)bridge('buddyStatus').then(setStatus).catch(e=>bubble(e.message));$('#buddy-question').focus();robot?.poke()}
function close(){panel.hidden=true;preview?.setVisible(false);$('#buddy-toggle').setAttribute('aria-expanded','false');$('#buddy-toggle').focus()}
function clearChat(message){epoch++;answer='';answerMailID=null;include.checked=false;log.replaceChildren();bubble(message);$('#use-buddy-draft').hidden=true;sharing()}
$('#buddy-toggle').onclick=()=>{if($('#buddy-toggle').dataset.dragged)return;panel.hidden?open():close()};$('#buddy-close').onclick=close;
$('#reader-buddy').onclick=()=>{$('#reader').close();open()};
$('#buddy-settings').onclick=async()=>{if(busy)return;try{setStatus(await bridge('buddySettings'));bubble('Settings saved. Ask a question to test the connection.')}catch(e){bubble(e.message)}};
$('#buddy-new-chat').onclick=()=>clearChat('A fresh start. What would you like help with?');
window.addEventListener('mail-account-changed',()=>clearChat('Account changed. Your previous conversation has been cleared.'));
include.onchange=sharing;
window.addEventListener('mail-selected',()=>{include.checked=false;sharing();$('#use-buddy-draft').hidden=!answer||(answerMailID!==null&&window.mailForBuddy()?.id!==answerMailID)});
const encouragement=['One thoughtful reply is enough to get started.','You are a person with an inbox, not an inbox with a person.','You can put something down without letting someone down.','Tiny progress still counts. I checked.','Your inbox does not get to decide how your whole day feels.'];
$('#buddy-cheer').onclick=()=>{bubble(encouragement[Math.floor(Math.random()*encouragement.length)]+'\n— A little Buddy note (not AI-generated)');robot?.poke()};
async function ask(question){
 if(busy)return;if(!native){bubble('Open the Mac app to connect Buddy to an AI.');return}if(!settings.ready){bubble('Choose Ollama, OpenAI, or Claude with the settings button first.');return}
 const selected=window.mailForBuddy(),context=include.checked&&selected?`From: ${selected.email}\nSubject: ${selected.subject}\n\n${selected.body}`:'';
 const sourceID=context?selected.id:null,requestEpoch=epoch,sentQuestion=question.trim();if(!sentQuestion)return;
 bubble(sentQuestion,'user');$('#buddy-question').value='';busy=true;$('#buddy-send').disabled=true;$('#buddy-settings').disabled=true;$('#use-buddy-draft').hidden=true;robot?.setState({busy:true});setStatus(settings);
 try{const result=await bridge('buddyAsk',{question:sentQuestion,context});if(epoch!==requestEpoch)return;answer=result.answer;answerMailID=sourceID;bubble(answer);$('#use-buddy-draft').hidden=sourceID!==null&&window.mailForBuddy()?.id!==sourceID;robot?.setState({happy:true})}
 catch(e){if(epoch===requestEpoch)bubble(e.message)}
 finally{busy=false;$('#buddy-send').disabled=false;$('#buddy-settings').disabled=false;robot?.setState({busy:false});setStatus(settings)}
}
$('#buddy-form').onsubmit=e=>{e.preventDefault();ask($('#buddy-question').value)};
$('#buddy-question').onkeydown=e=>{if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();ask(e.target.value)}};
panel.addEventListener('keydown',e=>{if(e.key==='Escape'){e.stopPropagation();close()}});
document.querySelectorAll('[data-ask]').forEach(b=>b.onclick=()=>{if(!window.mailForBuddy()){bubble('Open a message first. Then choose whether to include it.');return}if(!include.checked){bubble('Turn on “Include this email” to let me read the selected message.');include.focus();return}ask(b.dataset.ask)});
$('#use-buddy-draft').onclick=()=>{if(answerMailID!==null&&window.mailForBuddy()?.id!==answerMailID){bubble('That answer belongs to another email. Open that conversation before using it.');return}close();window.useBuddyDraft(answer,answerMailID)};
window.addEventListener('mail-session-complete',()=>{robot?.poke();bubble('A few small decisions, one lighter inbox. That is a good place to stop.\n— A little Buddy note (not AI-generated)')});
$('#buddy-name').onchange=e=>saveLook({name:e.target.value});
for(const key of Object.keys(BuddyLook.choices))$('#buddy-'+key).onchange=e=>saveLook({[key]:e.target.value});
for(const key of ['color','glow','outfit'])$('#buddy-'+key+'-custom').oninput=e=>saveLook({[key]:e.target.value});
for(const key of ['backpack','scarf'])$('#buddy-'+key).onchange=e=>saveLook({[key]:e.target.checked});
$('#buddy-reset').onclick=()=>saveLook(BuddyLook.defaults);
$('#buddy-photo-remove').onclick=()=>saveLook({image:''});
$('#buddy-photo').onchange=e=>{
 const file=e.target.files?.[0];e.target.value='';if(!file)return;
 if(!['image/png','image/jpeg','image/webp'].includes(file.type)||file.size>10*1024*1024){$('#buddy-look-status').textContent='Choose a PNG, JPEG, or WebP image under 10 MB.';return}
 const reader=new FileReader();reader.onerror=()=>$('#buddy-look-status').textContent='Could not read this picture.';
 reader.onload=()=>{const image=new Image();image.onerror=()=>$('#buddy-look-status').textContent='Could not open this picture.';image.onload=()=>{const canvas=document.createElement('canvas');canvas.width=canvas.height=160;const size=Math.min(image.width,image.height);canvas.getContext('2d').drawImage(image,(image.width-size)/2,(image.height-size)/2,size,size,0,0,160,160);saveLook({image:canvas.toDataURL('image/jpeg',.85)})};image.src=reader.result};reader.readAsDataURL(file);
};
$('#buddy-customize').addEventListener('toggle',()=>{
 const visible=$('#buddy-customize').open&&!panel.hidden;
 if(visible&&!preview&&window.Buddy3D&&!matchMedia('(prefers-reduced-motion: reduce)').matches){preview=window.Buddy3D.create($('#buddy-preview'),{interactive:true});preview?.setLook(look)}
 preview?.setVisible(visible);
});
window.mailBuddy={robot:()=>robot,open,close,prepareSelection(text){open();include.checked=false;$('#buddy-question').value='Explain this email excerpt in simple words:\n\n'+text.slice(0,1500);sharing();bubble('The highlighted text is ready below. Press Ask when you want to share it with your chosen AI.')}};
function mount(){if(robot||!window.Buddy3D||matchMedia('(prefers-reduced-motion: reduce)').matches)return;robot=window.Buddy3D.create($('#buddy-character'),{interactive:false});if(robot){$('#buddy-character').classList.add('has-robot');applyLook();robot.setState({sleepy:!settings.ready});let frame=0;document.addEventListener('pointermove',e=>{if(frame)return;frame=requestAnimationFrame(()=>{frame=0;const r=$('#buddy-character').getBoundingClientRect();robot.lookAt((e.clientX-r.x-r.width/2)/innerWidth,-(e.clientY-r.y-r.height/2)/innerHeight)})},{passive:true})}}
document.addEventListener('visibilitychange',()=>{robot?.setVisible(!document.hidden);preview?.setVisible(!document.hidden&&!panel.hidden&&$('#buddy-customize').open)});
window.addEventListener('buddy3d-ready',mount);mount();applyLook();
})();
