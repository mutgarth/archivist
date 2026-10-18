const {chromium}=require('playwright');
(async()=>{
 const browser=await chromium.launch({headless:true});
 const page=await browser.newPage({viewport:{width:1280,height:900}});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const base=process.env.ARCHIVIST_TEST_URL||'http://127.0.0.1:18098';
 const assets=await(await page.request.get(base+'/api/v1/assets')).json();
 const asset=assets.items.find(a=>a.pipeline_version==='video-timeline-v1'&&a.status==='completed');
 if(!asset)throw new Error('No completed temporal video');
 const data=await(await page.request.get(base+'/api/v1/assets/'+asset.id+'/segments')).json();
 if(data.items.filter(s=>s.kind==='visual').length<2)throw new Error('Multiple visual moments missing');
 let chosen,query;
 for(const segment of data.items.filter(s=>s.kind==='speech'&&s.start_ms>60000)){
  const words=segment.text.match(/\p{L}{4,}/gu)||[];
  if(words.length<3)continue;
  const q=words.slice(0,6).join(' ');
  const hits=await(await page.request.post(base+'/api/v1/search',{data:{query:q,mode:'text'}})).json();
  if(hits.items.some(a=>a.id===asset.id&&a.match?.start_ms===segment.start_ms)){chosen=segment;query=q;break;}
 }
 if(!chosen)throw new Error('No precise speech search result after one minute');
 await page.goto(base);
 await page.getByLabel('O que você procura?').fill(query);
 await page.getByLabel('Modo de busca').selectOption('text');
 await page.getByRole('button',{name:'Buscar',exact:true}).click();
 await page.locator('.card[data-start="'+chosen.start_ms+'"]').first().click();
 await page.waitForFunction(ms=>Math.abs(document.querySelector('#detail-video').currentTime-ms/1000)<.3,chosen.start_ms);
 await page.getByLabel('Tipo de trecho').selectOption('speech');
 if(await page.locator('.segment[data-kind="visual"]').count())throw new Error('Speech filter failed');
 const other=page.locator('.segment[data-kind="speech"]').last();
 const otherTime=Number(await other.getAttribute('data-start'));
 await other.click();
 await page.waitForFunction(ms=>Math.abs(document.querySelector('#detail-video').currentTime-ms/1000)<.3,otherTime);
 await page.setViewportSize({width:390,height:844});
 if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth))throw new Error('Horizontal overflow');
 if(errors.length)throw new Error(errors.join('\n'));
 console.log('PASS: speech search after 60s, result seek, transcript seek, filters, mobile.',{visual:data.items.filter(s=>s.kind==='visual').length,speech:data.items.filter(s=>s.kind==='speech').length});
 await browser.close();
})().catch(e=>{console.error(e);process.exit(1)});
