(function(root){
'use strict';
const Logic={
 bucket:m=>m.quiet || (m.newsletter||m.tag==='NEWSLETTER'?'later':'reply'),
 normalizedSubject:s=>String(s||'').replace(/^(?:(?:re|fw|fwd):\s*)+/gi,'').trim().toLowerCase(),
 replyMatch:(tracked,messages)=>messages.find(m=>m.id!==tracked.id&&m.remote&&String(m.email||'').toLowerCase()===String(tracked.email||'').toLowerCase()&&((tracked.messageID&&m.inReplyTo===tracked.messageID)||(Logic.normalizedSubject(m.subject)!==''&&Logic.normalizedSubject(m.subject)===Logic.normalizedSubject(tracked.subject)&&Number.isFinite(Date.parse(m.time))&&Date.parse(m.time)>(tracked.sentAt||tracked.waitingSince||Date.parse(tracked.time)||Infinity))))||null,
 possibleReply:(tracked,messages)=>Boolean(Logic.replyMatch(tracked,messages)),
 due:(m,now=Date.now())=>Boolean(m.followUp&&new Date(m.followUp+'T00:00:00').getTime()<=now),
 snoozed:(m,now=Date.now())=>Boolean(m.snoozedUntil&&Date.parse(m.snoozedUntil)>now),
 tomorrow:(now=new Date())=>{const d=new Date(now);d.setDate(d.getDate()+1);d.setHours(9,0,0,0);return d.toISOString()},
 radar:(m,messages,now=Date.now())=>Logic.possibleReply(m,messages)?'replied':Logic.due(m,now)?'due':'upcoming',
 merge:(old,incoming)=>{
  const priorByID=new Map(old.map(m=>[m.id,m]));
  const retained=old.filter(m=>!m.remote||m.folder!=='inbox'||m.followUp||m.quiet||m.star||Logic.snoozed(m));
  const map=new Map(retained.map(m=>[m.id,m]));
  for(const fresh of incoming){const prior=priorByID.get(fresh.id)||{};map.set(fresh.id,{...fresh,...Object.fromEntries(['quiet','followUp','waitingSince','localRead','localStar','folder','snoozedUntil'].filter(k=>Object.hasOwn(prior,k)).map(k=>[k,prior[k]])),unread:prior.localRead===undefined?fresh.unread:!prior.localRead,star:prior.localStar??fresh.star});}
  return [...map.values()].sort((a,b)=>(b.sentAt||Date.parse(b.time)||0)-(a.sentAt||Date.parse(a.time)||0));
 },
 escape:s=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]))
};root.MailLogic=Logic;if(typeof module!=='undefined')module.exports=Logic;
})(typeof window!=='undefined'?window:globalThis);
