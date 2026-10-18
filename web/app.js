const $=id=>document.getElementById(id);
let token=sessionStorage.getItem('archivist-token')||'',nextOffset=null,selected=null,urls=[],detailUrl=null,busy=false;
const status=message=>{$('status').textContent=message;};
const labels={queued:'Na fila',running:'Analisando',completed:'Pronta para busca',failed:'Análise falhou'};
async function api(path,body,extra={}){
 const r=await fetch(path,{method:body===undefined?'GET':'POST',headers:{Authorization:'Bearer '+token,...(body===undefined?{}:{'Content-Type':'application/json'}),...extra},body:body===undefined?undefined:JSON.stringify(body)});
 if(!r.ok){let code='Falha na solicitação';try{code=(await r.json()).error?.code||code;}catch{}
 const messages={unauthorized:'Chave inválida ou acesso expirado.',embedding_unavailable_use_text:'Embedding indisponível. Use “Somente texto” ou tente novamente.',invalid_input:'Confira os caminhos, o formato e a confirmação da cópia.',conflict:'Conflito: o arquivo foi alterado ou a operação já mudou de estado.',import_busy:'Outra importação está em andamento. Tente novamente.'};throw new Error(messages[code]||code);}
 return r;
}
async function blob(path){return URL.createObjectURL(await (await api(path)).blob());}
function loggedIn(value){$('login').hidden=value;$('workspace').hidden=!value;$('logout').hidden=!value;}
async function render(items,append=false){
 if(!append){urls.forEach(URL.revokeObjectURL);urls=[];$('grid').replaceChildren();}
 for(const asset of items){
 const card=document.createElement('button');card.className='card';card.type='button';
 const img=document.createElement('img');img.alt='Prévia de '+asset.name;img.loading='lazy';
 const div=document.createElement('div'),name=document.createElement('strong'),badge=document.createElement('span'),desc=document.createElement('p');
 name.textContent=asset.name;badge.className='badge';badge.textContent=labels[asset.status]||asset.status;
 desc.textContent=asset.description?asset.description.slice(0,150):'A imagem já está disponível. A descrição será adicionada após a análise.';
 div.append(name,badge,desc);card.append(img,div);card.addEventListener('click',()=>openDetail(asset.id).catch(e=>status(e.message)));$('grid').append(card);
 try{const url=await blob('/api/v1/assets/'+asset.id+'/preview');urls.push(url);img.src=url;}catch{img.alt='Prévia indisponível';}
 }
 $('count').textContent=$('grid').children.length?$('grid').children.length+' imagens exibidas':'Nenhuma imagem encontrada. Importe um lote ou tente outra busca.';
}
async function load(append=false){
 if(busy)return;busy=true;
 try{
 const query=$('query').value.trim();let data;
 if(query){data=await(await api('/api/v1/search',{query,mode:$('mode').value,limit:50})).json();nextOffset=null;}
 else{data=await(await api('/api/v1/assets?limit=25&offset='+(append?nextOffset||0:0))).json();nextOffset=data.next_offset;}
 await render(data.items,append);$('more').hidden=nextOffset===null;loggedIn(true);
 }finally{busy=false;}
}
async function openDetail(id){
 selected=await(await api('/api/v1/assets/'+id)).json();
 if(detailUrl)URL.revokeObjectURL(detailUrl);
 detailUrl=await blob('/api/v1/assets/'+id+'/preview');$('detail-image').src=detailUrl;
 $('detail-name').textContent=selected.name;$('detail-description').textContent=selected.description||'Análise ainda não concluída.';
 $('detail-meta').textContent=selected.width+' × '+selected.height+' · '+labels[selected.status]+(selected.vision_model?' · '+selected.vision_model:'');
 $('retry').hidden=selected.status!=='failed';$('detail-dialog').showModal();
}
$('login-form').addEventListener('submit',async e=>{e.preventDefault();token=$('token').value.trim();try{await load();sessionStorage.setItem('archivist-token',token);$('token').value='';status('');}catch(e){status(e.message);}});
$('logout').addEventListener('click',()=>{sessionStorage.removeItem('archivist-token');token='';urls.forEach(URL.revokeObjectURL);urls=[];$('grid').replaceChildren();loggedIn(false);status('Sessão encerrada.');});
$('search-form').addEventListener('submit',e=>{e.preventDefault();load().then(()=>status('')).catch(e=>status(e.message));});
$('all').addEventListener('click',()=>{$('query').value='';load().catch(e=>status(e.message));});
$('more').addEventListener('click',()=>load(true).catch(e=>status(e.message)));
$('open-import').addEventListener('click',()=>$('import-dialog').showModal());
$('close-import').addEventListener('click',()=>$('import-dialog').close());
$('close-detail').addEventListener('click',()=>$('detail-dialog').close());
let importKey=null,importBody=null;
$('import-form').addEventListener('submit',async e=>{
 e.preventDefault();const button=e.submitter;button.disabled=true;
 const body={files:$('paths').value.split('\n').map(s=>s.trim()).filter(Boolean),copy_confirmed:$('confirmed').checked};
 const serialized=JSON.stringify(body);if(serialized!==importBody){importKey=crypto.randomUUID();importBody=serialized;}
 try{const result=await(await api('/api/v1/imports',body,{'Idempotency-Key':importKey})).json();$('import-dialog').close();status(result.assets.length+' imagens registradas. A análise continuará em segundo plano.');importKey=null;importBody=null;$('query').value='';await load();}
 catch(e){status(e.message);$('import-dialog').close();}finally{button.disabled=false;}
});
$('download').addEventListener('click',async()=>{try{const url=await blob('/api/v1/assets/'+selected.id+'/original');const a=document.createElement('a');a.href=url;a.download=selected.name.split('/').pop();a.click();setTimeout(()=>URL.revokeObjectURL(url),10000);}catch(e){status(e.message);}});
$('retry').addEventListener('click',async()=>{try{await api('/api/v1/jobs/'+selected.job_id+'/retry',{});$('detail-dialog').close();status('Nova tentativa agendada.');await load();}catch(e){status(e.message);}});
if(token)load().catch(e=>{loggedIn(false);status(e.message);});
setInterval(()=>{if(token&&!$('workspace').hidden&&!$('query').value.trim()&&!$('detail-dialog').open&&!$('import-dialog').open)load().catch(()=>{});},15000);
