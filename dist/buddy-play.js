// Mail adaptation of The Stupid Browser's pick-up, throw, bounce and selection helper.
(()=>{
 const root=document.querySelector('#buddy-toggle'),panel=document.querySelector('#buddy-panel');
 const reduced=matchMedia('(prefers-reduced-motion: reduce)').matches;
 const p={x:0,y:0,vx:0,vy:0,free:false,raf:0,last:0};let down=null;
 const robot=()=>window.mailBuddy?.robot();
 function limits(){return {x:Math.max(0,innerWidth-root.offsetWidth),y:Math.max(0,innerHeight-root.offsetHeight-8)}}
 function place(){const lim=limits();p.x=Math.max(0,Math.min(lim.x,p.x));p.y=Math.max(0,Math.min(lim.y,p.y));root.style.left=p.x+'px';root.style.top=p.y+'px'}
 function stop(){cancelAnimationFrame(p.raf);p.raf=0;p.last=0;robot()?.setState({held:false,walking:false})}
 function home(){stop();down=null;p.free=false;root.classList.remove('free','held');root.style.left='';root.style.top='';robot()?.poke()}
 function frame(t){p.raf=0;if(down||!p.free)return;const dt=Math.min(.033,(t-(p.last||t-16))/1000);p.last=t;const lim=limits();p.vy+=2600*dt;p.x+=p.vx*dt;p.y+=p.vy*dt;if(p.x<0||p.x>lim.x)p.vx*=-.5;if(p.y<0)p.vy=Math.abs(p.vy)*.3;let rest=false;if(p.y>=lim.y){if(p.vy>320){robot()?.land(Math.min(1,p.vy/2200));p.vy*=-.42}else{p.vy=0;p.vx*=Math.pow(.86,dt*60);rest=Math.abs(p.vx)<6}}place();robot()?.setState({walking:p.y===lim.y&&Math.abs(p.vx)>40});if(rest){p.vx=0;robot()?.setState({walking:false});return}p.raf=requestAnimationFrame(frame)}
 root.addEventListener('pointerdown',e=>{if(e.button!==0)return;stop();const r=root.getBoundingClientRect();down={id:e.pointerId,x:e.clientX,y:e.clientY,ox:e.clientX-r.left,oy:e.clientY-r.top,moved:false,time:performance.now()};p.x=r.left;p.y=r.top;p.vx=0;p.vy=0;root.setPointerCapture(e.pointerId)});
 root.addEventListener('pointermove',e=>{if(!down||e.pointerId!==down.id)return;if(!down.moved&&Math.hypot(e.clientX-down.x,e.clientY-down.y)<6)return;if(!down.moved){down.moved=true;p.free=true;root.classList.add('free','held');window.mailBuddy?.close();robot()?.setState({held:true})}const now=performance.now(),dt=Math.max(.016,(now-down.time)/1000),x=e.clientX-down.ox,y=e.clientY-down.oy;p.vx=Math.max(-2500,Math.min(2500,(x-p.x)/dt));p.vy=Math.max(-2500,Math.min(2500,(y-p.y)/dt));p.x=x;p.y=y;down.time=now;place()});
 function release(e){if(!down||e.pointerId!==down.id)return;const moved=down.moved;down=null;root.classList.remove('held');robot()?.setState({held:false});if(moved){root.dataset.dragged='1';setTimeout(()=>delete root.dataset.dragged,0);if(!reduced&&e.type!=='pointercancel')p.raf=requestAnimationFrame(frame)}}
 root.addEventListener('pointerup',release);root.addEventListener('pointercancel',e=>{release(e);home()});root.addEventListener('lostpointercapture',()=>{if(down)home()});
 document.querySelector('#buddy-home').onclick=home;
 root.addEventListener('keydown',e=>{if(e.key==='Escape'){home();window.mailBuddy?.close()}});
 window.addEventListener('resize',()=>{if(p.free)place()});window.addEventListener('blur',()=>{if(down)home()});document.addEventListener('visibilitychange',()=>{if(document.hidden)home()});
 const speech=document.querySelector('#buddy-speech');let speechTimer;
 function say(text){document.querySelector('#buddy-speech-text').textContent=text;speech.hidden=false;clearTimeout(speechTimer);speechTimer=setTimeout(()=>speech.hidden=true,6500)}
 document.querySelector('#buddy-speech-close').onclick=()=>speech.hidden=true;
 root.addEventListener('dblclick',()=>{robot()?.poke();say('You can pick me up! Drag gently, or give me a little toss.')});
 window.addEventListener('mail-session-complete',()=>{robot()?.poke();say('A few small decisions. A little more room to breathe.')});
 const content=document.querySelector('#message-content'),selectButton=document.createElement('button');selectButton.className='buddy-selection';selectButton.textContent='Ask Buddy about highlighted text';selectButton.hidden=true;content.after(selectButton);let excerpt='';
 document.addEventListener('selectionchange',()=>{const selection=window.getSelection();excerpt=selection&&content.contains(selection.anchorNode)&&content.contains(selection.focusNode)?selection.toString().trim().slice(0,1500):'';selectButton.hidden=!excerpt});
 selectButton.addEventListener('mousedown',e=>e.preventDefault());selectButton.onclick=()=>{if(!excerpt)return;const text=excerpt;document.querySelector('#reader').close();selectButton.hidden=true;window.mailBuddy?.prepareSelection(text)};
 window.addEventListener('mail-account-changed',()=>{speech.hidden=true;selectButton.hidden=true;excerpt='';home()});
})();
