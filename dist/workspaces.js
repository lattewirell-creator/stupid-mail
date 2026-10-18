(()=>{
 const dialog=$('#workspace-dialog'),form=$('#workspace-share-form');let sharing=false;
 $('#open-workspaces').onclick=()=>{form.hidden=true;dialog.showModal()};
 $('#share-workspace').onclick=()=>{if(!current)return;$('#workspace-excerpt').value=`From: ${current.from} <${current.email}>\nSubject: ${current.subject}\n\n${current.body}`.slice(0,2500);$('#reader').close();form.hidden=false;dialog.showModal()};
 for(const provider of ['slack','teams'])for(const [button,action]of [['open','workspaceOpen'],['settings','workspaceSettings']])$('#'+provider+'-'+button).onclick=async()=>{try{await bridge(action,{provider});if(button==='settings')toast('Workspace sharing settings updated.')}catch(e){toast(e.message)}};
 form.onsubmit=async e=>{e.preventDefault();if(sharing)return;sharing=true;$('#workspace-send').disabled=true;try{await bridge('workspaceShare',{provider:$('#workspace-provider').value,text:$('#workspace-excerpt').value});dialog.close();toast('Accepted by the workspace. Check the destination for your message.')}catch(e){toast(e.message)}finally{sharing=false;$('#workspace-send').disabled=false}};
 window.addEventListener('mail-account-changed',()=>{dialog.close();$('#workspace-excerpt').value=''})
})();
