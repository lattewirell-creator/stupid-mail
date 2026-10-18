(() => {
let settings={ready:false},busy=false,answer='',answerMailID=null,robot=null,epoch=0;
const panel=$('#buddy-panel'),log=$('#buddy-log'),include=$('#buddy-include');
let look={name:'Buddy',color:'cream',shape:'hiker',hat:'none'};
try{look={...look,...JSON.parse(localStorage.getItem('stupid-mail-buddy-look')||'{}')}}catch{}
if(!['clay','mint','sky','grape','cream'].includes(look.color))look.color='clay';
look.name=String(look.name||'Buddy').slice(0,20);
function applyLook(){ $('#buddy-name').value=look.name;$('#buddy-color').value=look.color;$('#buddy-panel-title').textContent=look.name;$('.buddy-caption').textContent=look.name;$('#buddy-shape').value=look.shape||'hiker';$('#buddy-hat').value=look.hat||'none';robot?.setLook({color:look.color,shape:look.shape||'hiker',hat:look.hat||'none',glow:'amber'}) }
function bubble(text,who='buddy'){const p=document.createElement('p');p.className=who==='user'?'buddy-question':'buddy-answer';p.textContent=text;log.append(p);log.scrollTop=log.scrollHeight}
function sharing(){const m=window.mailForBuddy();const cloud=settings.provider&&settings.provider!=='ollama';$('#buddy-sharing').textContent=include.checked&&m?`${cloud?'Shares':'Uses locally'} “${m.subject.slice(0,55)}” with ${settings.provider||'your chosen AI'} when you ask.`:'No email is included. Buddy receives only your question.'}
function setStatus(s){settings=s;robot?.setState({sleepy:!s.ready});$('#buddy-provider').textContent=busy?'Thinking…':s.ready?`${s.provider==='ollama'?'Ollama · local':s.provider==='openai'?'OpenAI':'Claude'} · ${s.model}`:'Choose an AI in settings to wake Buddy up.';sharing()}
function open(){panel.hidden=false;$('#buddy-toggle').setAttribute('aria-expanded','true');sharing();if(native)bridge('buddyStatus').then(setStatus).catch(e=>bubble(e.message));$('#buddy-question').focus();robot?.poke()}
function close(){panel.hidden=true;$('#buddy-toggle').setAttribute('aria-expanded','false');$('#buddy-toggle').focus()}
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
for(const id of ['buddy-name','buddy-color','buddy-shape','buddy-hat'])$('#'+id).onchange=()=>{look={name:$('#buddy-name').value.trim().slice(0,20)||'Buddy',color:$('#buddy-color').value,shape:$('#buddy-shape').value,hat:$('#buddy-hat').value};try{localStorage.setItem('stupid-mail-buddy-look',JSON.stringify(look))}catch{}applyLook()};
window.mailBuddy={robot:()=>robot,open,close,prepareSelection(text){open();include.checked=false;$('#buddy-question').value='Explain this email excerpt in simple words:\n\n'+text.slice(0,1500);sharing();bubble('The highlighted text is ready below. Press Ask when you want to share it with your chosen AI.')}};
function mount(){if(robot||!window.Buddy3D||matchMedia('(prefers-reduced-motion: reduce)').matches)return;robot=window.Buddy3D.create($('#buddy-character'),{interactive:false});if(robot){$('#buddy-character').classList.add('has-robot');applyLook();let frame=0;document.addEventListener('pointermove',e=>{if(frame)return;frame=requestAnimationFrame(()=>{frame=0;const r=$('#buddy-character').getBoundingClientRect();robot.lookAt((e.clientX-r.x-r.width/2)/innerWidth,-(e.clientY-r.y-r.height/2)/innerHeight)})},{passive:true})}}
document.addEventListener('visibilitychange',()=>robot?.setVisible(!document.hidden));
window.addEventListener('buddy3d-ready',mount);mount();applyLook();
})();
