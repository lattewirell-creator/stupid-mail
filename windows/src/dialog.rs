// Native-style dialogs for the Windows version (the Mac app's NSAlert sheets): each is its own small window,
// so passwords, tokens and API keys typed into it go straight to the app, never through the mail interface.
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Field { pub id: &'static str, pub kind: &'static str, pub placeholder: String, pub value: String, pub options: Vec<String> }
impl Field {
    pub fn text(id: &'static str, placeholder: &str, value: &str) -> Field { Field { id, kind: "text", placeholder: placeholder.into(), value: value.into(), options: vec![] } }
    pub fn secret(id: &'static str, placeholder: &str) -> Field { Field { id, kind: "password", placeholder: placeholder.into(), value: String::new(), options: vec![] } }
    pub fn choice(id: &'static str, options: &[&str], selected: usize) -> Field { Field { id, kind: "select", placeholder: String::new(), value: selected.to_string(), options: options.iter().map(|s| s.to_string()).collect() } }
    pub fn readonly(id: &'static str, value: &str) -> Field { Field { id, kind: "readonly", placeholder: String::new(), value: value.into(), options: vec![] } }
    pub fn preview(id: &'static str, value: &str) -> Field { Field { id, kind: "preview", placeholder: String::new(), value: value.into(), options: vec![] } }
}
#[derive(Serialize, Clone)]
pub struct Spec { pub title: String, pub text: String, pub fields: Vec<Field>, pub buttons: Vec<String> }
impl Spec {
    pub fn new(title: &str, text: &str, buttons: &[&str]) -> Spec { Spec { title: title.into(), text: text.into(), fields: vec![], buttons: buttons.iter().map(|s| s.to_string()).collect() } }
    pub fn field(mut self, f: Field) -> Spec { self.fields.push(f); self }
    pub fn height(&self) -> f64 {
        let lines = self.text.len() as f64 / 62.0 + self.text.matches('\n').count() as f64;
        180.0 + lines * 21.0 + self.fields.iter().map(|f| if f.kind == "preview" { 190.0 } else { 46.0 }).sum::<f64>()
    }
}

/// The dialog page: the app's cream-and-terracotta look; first button is the default (Enter), Escape picks the last
/// "Cancel"-style button. Replies with window.ipc.postMessage({button, values}).
pub fn html(spec: &Spec) -> String {
    let data = serde_json::to_string(spec).unwrap().replace("</", "<\\/");
    format!(r##"<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'">
<style>
:root{{--bg:#FAF7F1;--ink:#2B2824;--muted:#7D776D;--line:#E4DDD1;--clay:#D97757;--field:#fff}}
@media (prefers-color-scheme: dark){{:root{{--bg:#22201D;--ink:#EEE9E0;--muted:#A39D93;--line:#3A3631;--field:#2C2925}}}}
*{{box-sizing:border-box}} body{{margin:0;padding:22px 24px 18px;font:14px/1.45 "Segoe UI",system-ui,sans-serif;background:var(--bg);color:var(--ink);user-select:none}}
h1{{font:600 19px Georgia,"Times New Roman",serif;margin:0 0 8px}} p{{margin:0 0 14px;color:var(--muted);white-space:pre-wrap}}
input,select,textarea{{width:100%;margin:0 0 10px;padding:9px 11px;border:1px solid var(--line);border-radius:9px;background:var(--field);color:var(--ink);font:inherit;user-select:text}}
input:focus,select:focus{{outline:2px solid color-mix(in srgb,var(--clay) 55%,transparent);border-color:var(--clay)}}
textarea{{height:170px;resize:none}} .code{{font:600 22px Consolas,monospace;letter-spacing:.12em;text-align:center}}
.row{{display:flex;gap:8px;justify-content:flex-end;margin-top:8px;flex-wrap:wrap}}
button{{padding:8px 16px;border-radius:999px;border:1px solid var(--line);background:transparent;color:var(--ink);font:inherit;cursor:pointer}}
button.main{{background:var(--clay);border-color:var(--clay);color:#fff}} button:focus-visible{{outline:2px solid var(--clay);outline-offset:2px}}
</style></head><body><h1 id="t"></h1><p id="x"></p><form id="f"></form><div class="row" id="b"></div>
<script>
const s={data};document.title=s.title;t.textContent=s.title;x.textContent=s.text;
for(const d of s.fields){{let e;
 if(d.kind==='select'){{e=document.createElement('select');d.options.forEach((o,i)=>{{const op=new Option(o,String(i));e.add(op)}});e.value=d.value}}
 else if(d.kind==='preview'){{e=document.createElement('textarea');e.readOnly=true;e.value=d.value}}
 else{{e=document.createElement('input');e.type=d.kind==='password'?'password':'text';e.value=d.value;e.placeholder=d.placeholder;e.spellcheck=false;e.autocomplete='off';if(d.kind==='readonly'){{e.readOnly=true;e.className='code'}}}}
 e.id='f_'+d.id;f.appendChild(e)}}
const send=i=>{{const v={{}};for(const d of s.fields)if(d.kind!=='preview'&&d.kind!=='readonly')v[d.id]=document.getElementById('f_'+d.id).value;window.ipc.postMessage(JSON.stringify({{button:i,values:v}}))}};
s.buttons.forEach((l,i)=>{{const e=document.createElement('button');e.textContent=l;if(i===0)e.className='main';e.onclick=()=>send(i);b.appendChild(e)}});
f.onsubmit=e=>{{e.preventDefault();send(0)}};
addEventListener('keydown',e=>{{if(e.key==='Enter'&&e.target.tagName!=='TEXTAREA'){{e.preventDefault();send(0)}}if(e.key==='Escape'){{const c=s.buttons.findIndex(l=>/cancel/i.test(l));send(c<0?s.buttons.length-1:c)}}}});
(document.querySelector('input:not([readonly]),select')||document.querySelector('button.main')).focus();
</script></body></html>"##)
}
