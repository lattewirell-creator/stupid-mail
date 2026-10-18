// Normal Buddy defaults and customization catalog from The Stupid Browser 9234994.
(()=>{
const defaults={name:'Buddy',color:'cream',glow:'cyan',shape:'hiker',hat:'none',glasses:'none',image:'',eyes:'dot',mouth:'smile',outfit:'clay',backpack:true,scarf:true};
const colors={cream:'#EBC39D',clay:'#D97757',cocoa:'#8B5E45',sky:'#4F8DE0',mint:'#3FB98A',grape:'#8A5CD6',rose:'#E0607E',sun:'#E9A23B',night:'#3B3F58',snow:'#D9DCE3'};
const glows={cyan:'#7CF5FF',green:'#7CFF9E',pink:'#FF8AD8',amber:'#FFC86B',white:'#FFFFFF',violet:'#B79CFF'};
const choices={color:Object.keys(colors),glow:Object.keys(glows),shape:['hiker','ball','round','cube','capsule','egg','tv'],eyes:['dot','round','sleepy','wide','lash'],mouth:['smile','grin','flat','open','cat'],outfit:Object.keys(colors),hat:['none','tophat','cap','beanie','crown','party','cowboy','headphones','halo'],glasses:['none','round','square','shades','star','heart']};
function normalize(raw={}){const look={...defaults};look.name=String(raw.name||defaults.name).trim().slice(0,20)||defaults.name;for(const key in choices)if(choices[key].includes(raw[key])||(['color','glow','outfit'].includes(key)&&/^#[0-9a-f]{6}$/i.test(raw[key]||'')))look[key]=raw[key];for(const key of ['backpack','scarf'])look[key]=raw[key]!==false;if(typeof raw.image==='string'&&raw.image.length<200000&&/^data:image\/(png|jpeg|webp);base64,[A-Za-z0-9+/=]+$/.test(raw.image))look.image=raw.image;return look}
window.BuddyLook={defaults,colors,glows,choices,normalize};
})();
