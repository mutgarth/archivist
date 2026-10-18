const $=id=>document.getElementById(id);
let nextOffset=null,selected=null,urls=[],detailUrl=null,busy=false;
const status=message=>{$('status').textContent=message;};
const labels={queued:'Na fila',running:'Analisando',completed:'Pronta para busca',failed:'Análise falhou'};
async function api(path,body,extra={}){
 const r=await fetch(path,{method:body===undefined?'GET':'POST',headers:{...(body===undefined?{}:{'Content-Type':'application/json'}),...extra},body:body===undefined?undefined:JSON.stringify(body)});
 if(!r.ok){let code='Falha na solicitação';try{code=(await r.json()).error?.code||code;}catch{}
 const messages={storage_unavailable:'Armazenamento indisponível ou volume incorreto. Confira a montagem antes de importar.',embedding_unavailable_use_text:'Embedding indisponível. Use “Somente texto” ou tente novamente.',invalid_input:'Confira os caminhos, o formato e a confirmação da cópia.',conflict:'Conflito: o arquivo foi alterado ou a operação já mudou de estado.',import_busy:'Outra importação está em andamento. Tente novamente.'};throw new Error(messages[code]||code);}
 return r;
}
async function blob(path){return URL.createObjectURL(await (await api(path)).blob());}
async function render(items,append=false){
 if(!append){urls.forEach(URL.revokeObjectURL);urls=[];$('grid').replaceChildren();}
 for(const asset of items){
 const card=document.createElement('button');card.className='card';card.dataset.name=asset.name;card.dataset.mime=asset.mime;card.type='button';
 const img=document.createElement('img');img.alt='Prévia de '+asset.name;img.loading='lazy';
 const div=document.createElement('div'),name=document.createElement('strong'),badge=document.createElement('span'),desc=document.createElement('p');
 name.textContent=(asset.mime.startsWith('video/')?'▶ ':'')+asset.name;badge.className='badge';badge.textContent=labels[asset.status]||asset.status;
 desc.textContent=asset.description?asset.description.slice(0,150):'O original já está disponível. A descrição será adicionada após a análise.';
 div.append(name,badge,desc);card.append(img,div);card.addEventListener('click',()=>openDetail(asset.id).catch(e=>status(e.message)));$('grid').append(card);
 try{const url=await blob('/api/v1/assets/'+asset.id+'/preview');urls.push(url);img.src=url;}catch{img.alt='Prévia indisponível';}
 }
 $('count').textContent=$('grid').children.length?$('grid').children.length+' arquivos exibidos':'Nenhum arquivo encontrado. Importe um lote ou tente outra busca.';
}
async function load(append=false){
 if(busy)return;busy=true;
 try{
 const query=$('query').value.trim();let data;
 if(query){data=await(await api('/api/v1/search',{query,mode:$('mode').value,limit:50})).json();nextOffset=null;}
 else{data=await(await api('/api/v1/assets?limit=25&offset='+(append?nextOffset||0:0))).json();nextOffset=data.next_offset;}
 await render(data.items,append);$('more').hidden=nextOffset===null;
 }finally{busy=false;}
}
async function openDetail(id){
 selected=await(await api('/api/v1/assets/'+id)).json();
 if(detailUrl)URL.revokeObjectURL(detailUrl);
 detailUrl=await blob('/api/v1/assets/'+id+'/preview');$('detail-image').src=detailUrl;
 const isVideo=selected.mime.startsWith('video/');
 $('detail-video').hidden=!isVideo;$('video-note').hidden=!isVideo;$('detail-image').hidden=isVideo;
 $('detail-video').removeAttribute('src');
 if(isVideo){$('video-note').textContent='Prévia reduzida para reprodução. O download preserva o original integral. A descrição considera apenas um quadro.';$('detail-video').poster=detailUrl;$('detail-video').src='/api/v1/assets/'+id+'/playback';}
 $('detail-name').textContent=selected.name;$('detail-description').textContent=selected.description||'Análise ainda não concluída.';
 $('detail-meta').textContent=selected.width+' × '+selected.height+' · '+labels[selected.status]+(selected.mime.startsWith('video/')?' · '+Math.round(Number(selected.media_metadata?.format?.duration||0))+' s · '+(selected.bytes/1e9).toFixed(2)+' GB':'')+(selected.vision_model?' · '+selected.vision_model:'');
 $('detail-integrity').textContent='SHA-256 do original: '+selected.sha256;
 if(isVideo){const stream=selected.media_metadata?.streams?.find(s=>s.codec_type==='video');if(stream)$('detail-meta').textContent+=' · '+stream.codec_name+' · '+stream.r_frame_rate+' fps';}
 $('retry').hidden=selected.status!=='failed';$('detail-dialog').showModal();
}
$('search-form').addEventListener('submit',e=>{e.preventDefault();load().then(()=>status('')).catch(e=>status(e.message));});
$('all').addEventListener('click',()=>{$('query').value='';load().catch(e=>status(e.message));});
$('more').addEventListener('click',()=>load(true).catch(e=>status(e.message)));
$('open-import').addEventListener('click',()=>$('import-dialog').showModal());
$('close-import').addEventListener('click',()=>$('import-dialog').close());
$('close-detail').addEventListener('click',()=>$('detail-dialog').close());
$('detail-dialog').addEventListener('close',()=>{$('detail-video').pause();$('detail-video').removeAttribute('src');$('detail-video').load();});
$('detail-video').addEventListener('error',()=>{if(!$('detail-video').hidden)$('video-note').textContent='Prévia ainda indisponível. A preparação ocorre em segundo plano; reabra o vídeo em alguns instantes. O original já pode ser baixado.';});
let importKey=null,importBody=null;
$('import-form').addEventListener('submit',async e=>{
 e.preventDefault();const button=e.submitter;button.disabled=true;$('import-progress').textContent='Copiando e verificando os originais. Arquivos grandes podem levar alguns minutos.';
 const body={files:$('paths').value.split('\n').map(s=>s.trim()).filter(Boolean),copy_confirmed:$('confirmed').checked};
 const serialized=JSON.stringify(body);if(serialized!==importBody){importKey=crypto.randomUUID();importBody=serialized;}
 try{const result=await(await api('/api/v1/imports',body,{'Idempotency-Key':importKey})).json();$('import-dialog').close();status(result.assets.length+' arquivos registrados. A análise continuará em segundo plano.');importKey=null;importBody=null;$('query').value='';await load();}
 catch(e){status(e.message);$('import-dialog').close();}finally{button.disabled=false;$('import-progress').textContent='';}
});
$('download').addEventListener('click',()=>{const a=document.createElement('a');a.href='/api/v1/assets/'+selected.id+'/original';a.download=selected.name.split('/').pop();a.click();});
$('retry').addEventListener('click',async()=>{try{await api('/api/v1/jobs/'+selected.job_id+'/retry',{});$('detail-dialog').close();status('Nova tentativa agendada.');await load();}catch(e){status(e.message);}});
sessionStorage.removeItem('archivist-token');
load().catch(e=>status(e.message));
setInterval(()=>{if(!$('query').value.trim()&&!$('detail-dialog').open&&!$('import-dialog').open)load().catch(()=>{});},15000);
