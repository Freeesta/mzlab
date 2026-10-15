// @ts-strict
// Page shell: helpers shared with explore.js (classic scripts share globals), start screen, views.
const $=s=>document.querySelector(s);
// addresses of the views: #/dati, #/disegno (and #/teoria, which the head script above sends to the standalone Teoria on a cold start)
const fmt=v=>v===0?"0":(Math.abs(v)>=1e4||Math.abs(v)<.01)?v.toExponential(1):(+v.toPrecision(3)).toString();
const css=n=>getComputedStyle(document.documentElement).getPropertyValue(n).trim();
const esc=t=>String(t).replace(/[&<>"]/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}[c]));
// fmtFormula / fmtAdduct (subscripts, superscripts) are defined in explore.js, loaded before this script
async function api(u){const r=await fetch(u);const j=await r.json();if(j.error)throw I18N.err(j);return j}
function fail(e){$("#err").textContent=e?(e.message||e):""}
// canvas resolution = device pixels x the pinch scale (a "visual" zoom on a Mac trackpad), at most 4; every canvas is redrawn when it changes
function dprNow(){const s=(window.visualViewport&&window.visualViewport.scale)||1;return Math.min(4,(devicePixelRatio||1)*s)}
{let last=dprNow(),t=0;const redo=()=>{const d=dprNow();if(Math.abs(d-last)<.01)return;last=d;if(typeof redrawAll==="function")redrawAll()};
 const arm=()=>{try{matchMedia(`(resolution: ${devicePixelRatio}dppx)`).addEventListener("change",()=>{redo();arm()},{once:true})}catch(_){}};arm();   // browser zoom (Ctrl/Cmd +)
 addEventListener("resize",()=>{clearTimeout(t);t=setTimeout(redo,60)});
 if(window.visualViewport)window.visualViewport.addEventListener("resize",()=>{clearTimeout(t);t=setTimeout(redo,150)})}   // pinch: after a short pause
function setup(c){const z=typeof fz==="function"?fz():1;M.l=Math.max(Math.round(66*z),window.MARG_L||0);M.r=Math.max(14,window.MARG_R||0);M.t=Math.round(14*z);M.b=Math.round(40*z);const r={width:c.clientWidth,height:c.clientHeight},d=dprNow();c.width=Math.round(r.width*d);c.height=Math.round(r.height*d);const g=c.getContext("2d");g.setTransform(d,0,0,d,0,0);return{g,W:r.width,H:r.height}}   // layout size, not getBoundingClientRect: a panel sliding in (transizioni.js scales it) must not change the resolution
function nice(a,b,n){const s=(b-a)/n,p=Math.pow(10,Math.floor(Math.log10(s))),m=s/p,st=(m<1.5?1:m<3?2:m<7?5:10)*p,o=[];for(let t=Math.ceil(a/st)*st;t<=b+1e-9;t+=st)o.push(+t.toFixed(10));return o}
function smooth(y,k){if(!k)return y;return y.map((_,i)=>{const a=y[i-1]??y[i],c=y[i+1]??y[i];return(a+2*y[i]+c)/4})}
const M={l:66,r:14,t:14,b:40};   // plot margins (px): room for tick labels and axis titles

async function init(){try{loading(true);await bootSession();await Promise.all(E.panels.map(p=>p.ready).filter(Boolean)).catch(()=>{});
  const d=await api("api/state");fail();window.qqReady=true;setTimeout(prefetchIdle,3000);const hasSess=!!(d.app||S.sess);$("#newrun").hidden=!hasSess;if(!S.sess)showStart(d);else{applyView();if(window.QQRESET)QQRESET.remind()}if(window.BARRA)BARRA.init()}catch(e){fail(e)}finally{loading(false)}}

// ------------------------------------------------------------------ views: Dati (start screen or panels), Disegno
let _lastV="";
function applyView(){const v=S.view,d=v==="data";
  $("#start").hidden=!(d&&(!S.sess||S.adding));$("#v-data").hidden=!(d&&S.sess&&!S.adding);$("#cancadd").hidden=!(S.sess&&S.adding);renderLoaded();
  if(window.BARRA){if(d&&S.sess&&!S.adding)BARRA.dockCalc(true);else BARRA.dockCalc(false);}
  $("#v-draw").hidden=v!=="draw";document.documentElement.classList.toggle("in-draw",v==="draw");$("#v-theory").hidden=v!=="theory";
  // The Teoria is Italian only: in English it opens at once in Italian, with a short toast each time the student enters it
  if(v==="theory"&&!$("#tframe").getAttribute("src"))$("#tframe").src="static/teoria/index.html"; // theory module, loaded on first open
  if(v==="theory"&&_lastV!=="theory"&&I18N.lang==="en"&&window.toast)toast(I18N.t("theory.onlyItalian"));
  _lastV=v;
  document.querySelectorAll("#nav button").forEach(b=>b.classList.toggle("on",b.dataset.v===v));
  document.title=(v==="data"?I18N.t("app.title",{app:window.APP_NAME||""}):I18N.t("nav."+(v==="draw"?"draw":"theory"))+" · "+(window.APP_NAME||""));   // one title per view (WCAG 2.4.2)
  document.dispatchEvent(new CustomEvent("tpview",{detail:{view:v}}))}
// the header stays in view (sticky): --hh is its real height (it can wrap on a narrow window), used by the file list, the full-screen panel and the scrolls to a panel
{const hd=document.querySelector("header"),hh=()=>{document.documentElement.style.setProperty("--hh",hd.offsetHeight+"px");hd.classList.toggle("scrolled",scrollY>4)};
 hh();addEventListener("scroll",()=>hd.classList.toggle("scrolled",scrollY>4),{passive:true});addEventListener("resize",hh);if(window.ResizeObserver)new ResizeObserver(hh).observe(hd)}
window.hdrH=()=>document.querySelector("header").offsetHeight;
function setView(v,fromHistory){STATO.vista(v,"view");applyView();if(!fromHistory&&VIEW_HASH[v]&&location.hash!=="#/"+VIEW_HASH[v])history.pushState(null,"","#/"+VIEW_HASH[v]);   // Back and Forward walk through the views
  if(v==="data"&&!window.qqReady&&!window.QQQ_PHONE)$("#loading").hidden=false}   // no automatic scroll when the view changes (decision 7/10): panels, zoom, cursor and active graph are state and stay
document.querySelectorAll("#nav button").forEach(b=>b.onclick=()=>setView(b.dataset.v));
// Back/Forward and typed addresses: the view follows the hash (a cold #/teoria never gets here: the head script leaves the page)
addEventListener("popstate",()=>{const v=hashView()||"data";if(v!==S.view)setView(v,true)});
// a direct address shows its view at once, without waiting for the engine (the loading veil stays until the program is ready)
if(S.view!=="data"){$("#v-draw").hidden=S.view!=="draw";$("#v-theory").hidden=S.view!=="theory";document.documentElement.classList.toggle("in-draw",S.view==="draw");document.querySelectorAll("#nav button").forEach(b=>b.classList.toggle("on",b.dataset.v===S.view))}

// ------------------------------------------------------------------ start screen: upload, check times and types, open
const sz=b=>b>1e6?(b/1e6).toFixed(1)+" MB":Math.max(1,Math.round(b/1e3))+" kB";
async function post(u,body,raw){const r=await fetch(u,{method:"POST",body:raw??JSON.stringify(body||{})});const j=await r.json();if(j.error)throw I18N.err(j);return j}
// concentration units: key -> [label, factor to mg/L]; the names carry the unit (std_0.5ppm), the table shows the one chosen in its header
const CU={mgl:["mg/L",1],ugl:["µg/L",1e-3],ngl:["ng/L",1e-6],gl:["g/L",1e3]},CUN={ppm:"mgl",ppb:"ugl",ppt:"ngl",mgl:"mgl",ugl:"ugl",ngml:"ugl",ngl:"ngl",gl:"gl"};
const CUF={ppm:1,ppb:1e-3,ppt:1e-6,mgl:1,ugl:1e-3,ngml:1e-3,ngl:1e-6,gl:1e3};
ST.unit="mgl";
const roundc=v=>v==null?null:+(+v).toPrecision(6);
const isCal=f=>f.type==="standard";   // concentration exists only for standards (in an MRM experiment they feed the calibration line)
const concFrom=f=>f.conc==null||f.kind!=="mrm"?null:roundc(f.cunit&&CUF[f.cunit]?f.conc*CUF[f.cunit]/CU[ST.unit][1]:f.conc);   // value from the name, in the unit of the table
function mergeFiles(list){const old=Object.fromEntries(ST.files.map(f=>[f.name,f]));
  ST.files=list.map(f=>{const o=old[f.name],type=o?o.type:f.type,conc=o?o.conc:concFrom(f),time=type==="blank"?null:o?o.time:f.time;return {...f,time,type,conc,use:o?o.use:true,
    gC:o?o.gC:conc!=null,role:o&&o.role,prop:o&&o.prop,ed:o&&o.ed}});   // gC: concentration read from the name (yellow in the table until the student edits or confirms them)
  sortFiles();renderFiles();RUOLI.propose(()=>ST.files).then(c=>{if(c){sortFiles();renderFiles()}})}   // roles and times proposed from the names by the engine (campioni/2)
// order of the table: experiment (Full Scan, MS2, MRM, the rest), then blank, standards, samples; inside each, the rule of ordine.js (time, else concentration, else natural name)
const EXPORD={full:0,ms2:1,mrm:2,infusion:3},TYPORD={blank:0,standard:1,sample:2};
function sortFiles(){const c=fileComparator(ST.files,f=>f.conc==null?null:f.conc*CU[ST.unit][1]);ST.files.sort((a,b)=>(EXPORD[a.kind]??4)-(EXPORD[b.kind]??4)||TYPORD[a.type]-TYPORD[b.type]||c(a,b))}
// the formats of instrument methods the program reads TODAY (one line per format; the field, the file picker and the texts all come from here)
const METHOD_FORMATS=[{ext:".dam",maker:"SCIEX"}];
const isMethodFile=name=>METHOD_FORMATS.some(x=>name.toLowerCase().endsWith(x.ext));
(()=>{const fm=METHOD_FORMATS.map(x=>x.ext+" "+x.maker).join(", ");$("#pickdam").accept=METHOD_FORMATS.map(x=>x.ext).join(",");
  $("#dropdam").setAttribute("aria-label",I18N.t("load.method.drop",{formats:fm}));$("#dropdam").innerHTML=`<b>${I18N.t("load.method.drop",{formats:fm})}</b><br>${I18N.t("load.method.dropHint")}`})();
// each method is a chip «Method: name» with ×; the method goes with the data file of the same base name, else with the selected file (api.method)
function renderMethods(){const el=$("#mlist");
  el.innerHTML=ST.methods.map((m,i)=>`<span class="chip"><span>${I18N.t("load.method.chip",{name:esc(m.name)})}</span> <span class="muted sm">${esc(m.info&&m.info.key?I18N.t(m.info.key,m.info.params):"")} · ${sz(m.size)}</span> <button type="button" data-mrm="${i}" aria-label="${esc(I18N.t("load.method.remove",{name:m.name}))}" title="${I18N.t("load.table.remove")}">&times;</button></span>`).join("");
  el.querySelectorAll("[data-mrm]").forEach(b=>b.onclick=async()=>{try{const j=await post("api/remove",{name:ST.methods[+b.dataset.mrm].name});mergeFiles(j.files);ST.methods=j.methods||[];renderMethods()}catch(e){fail(e)}})}
const EXPN={full:"Full Scan",ms2:"MS2 (Product Ion)",mrm:"MRM",infusion:I18N.t("inf.name")},expn=k=>k==="empty"?I18N.t("load.exp.empty"):EXPN[k]||k;
const mixedText=m=>{const ns=[];for(const p of m){const n=p.name==="MS2"&&ns.includes("Full Scan")?"DDA":p.name;if(!ns.includes(n))ns.push(n)}return ns.join(" + ")};   // Full Scan + MS2 = DDA; polarities and precursor counts are not shown
const expCell=f=>f.kind&&f.mixed&&f.mixed.length?`<b>${esc(mixedText(f.mixed))}</b>`:f.kind?`<b>${expn(f.kind)}</b>${f.polarity==="mixed"?` <span class="muted sm">${I18N.t("load.exp.polMixed")}</span>`:""}`:`<span class="muted">${I18N.t("load.exp.unread")}</span>`;
function nameWarn(f){return f.kind&&!f.mixed&&f.hint&&f.hint!==f.kind?` <div class="fail sm">${I18N.t("load.exp.nameHint",{hint:expn(f.hint),kind:expn(f.kind)})}</div>`:""}
function renderFiles(){const F=ST.files,el=$("#flist");
  const hasCal=F.some(f=>f.use&&isCal(f)),hasTime=F.some(f=>f.time!=null),tu=tunitGet(),tuSel=`<select id="tunit" title="${I18N.t("load.table.tunit.title")}" aria-label="${I18N.t("load.table.tunit.title")}">${Object.keys(TIME_UNITS).map(k=>`<option value="${k}" ${k===tu?"selected":""}>${tunitLabel(k)}</option>`).join("")}</select>`,unitSel=`<select id="cunit" title="${I18N.t("load.table.unit.title")}">${Object.entries(CU).map(([k,v])=>`<option value="${k}" ${k===ST.unit?"selected":""}>${v[0]}</option>`).join("")}</select>`;
  const ncol=7+(hasCal?1:0),GN={0:"Full Scan",1:"MS2 (Product Ion)",2:"MRM",3:I18N.t("inf.name"),4:I18N.t("load.group.other")},GUESS=` title="${I18N.t("load.guess.title")}"`;   // GUESS: concentration cells
  let last=-1,rows="";
  F.forEach((f,i)=>{const g=EXPORD[f.kind]??4;
    if(g!==last){last=g;rows+=`<tr class="grp"><td colspan="${ncol}">${GN[g]}${typeof polHead==="function"&&g<3?polHead(F.filter(x=>(EXPORD[x.kind]??4)===g)):""} <span class="muted">${I18N.t("load.group.count",{n:F.filter(x=>(EXPORD[x.kind]??4)===g).length})}</span></td></tr>`}
    rows+=`<tr><td><input type="checkbox" data-i="${i}" data-k="use" ${f.use?"checked":""}></td><td>${esc(f.name)} ${typeof polSign==="function"?polSign(f,F):""}${nameWarn(f)}</td><td>${expCell(f)}</td>`+
   RUOLI.roleCell(f,i)+RUOLI.timeCell(f,i,tu)+
   (hasCal?(isCal(f)?`<td class="num"><input type="text" inputmode="decimal" autocomplete="off" style="width:90px" data-i="${i}" data-k="conc" value="${f.conc??""}" ${f.gC&&f.conc!=null?`class="guess"${GUESS}`:`title="${I18N.t("load.table.concStd.title")}"`}></td>`:f.type==="blank"?`<td class="num"><input type="text" disabled aria-label="${I18N.t("load.table.conc")}" style="width:90px" data-i="${i}" data-k="conc" value=""></td>`:"<td></td>"):"")+
   `<td class="num muted">${sz(f.size)}</td><td style="white-space:nowrap">${CONV.has(f.name)?`<button type="button" class="sec sm" data-dl="${esc(f.name)}" title="${I18N.t("conv.dl.title")}">${I18N.t("conv.dl")}</button> `:""}<button data-rm="${i}" title="${I18N.t("load.table.remove")}">&times;</button></td></tr>`});
  rows+=CONVROWS.html(ncol);
  el.innerHTML=F.length||CONVROWS.rows.length?(CONV.size>1?`<div style="margin-top:8px"><button type="button" class="sec sm" id="dlall" title="${I18N.t("conv.dl.title")}">${I18N.t("conv.dlall")}</button></div>`:"")+`<table style="margin-top:8px"><tr><th></th><th scope="col">${I18N.t("load.table.file")}</th><th scope="col">${I18N.t("load.table.experiment")}</th><th scope="col">${I18N.t("load.table.type")}</th><th scope="col" class="num">${I18N.t("load.table.time.col")}${hasTime?` ${tuSel}`:""}</th>${hasCal?`<th scope="col" class="num">${I18N.t("load.table.conc")} ${unitSel}</th>`:""}<th scope="col" class="num">${I18N.t("load.table.size")}</th><th></th></tr>`+rows+`</table>`:"";
  el.querySelectorAll("[data-k]").forEach(x=>{
    x.onchange=()=>{const f=F[+x.dataset.i],k=x.dataset.k,keep=document.activeElement===x?{name:f.name,k}:null;let v=x.value;
      if(k==="time"||k==="conc"){const dk=k==="time"&&RUOLI.roleOf(f)==="dark";v=parseDecimal(x.value,dk);if(Number.isNaN(v)){x.setAttribute("aria-invalid","true");x.title=I18N.t("load.table.num.bad");x.classList.add("bad");return}x.removeAttribute("aria-invalid");if(k==="time")v=dk?darkTime(timeStored(v,tunitGet())):timeStored(v,tunitGet())}
      if(k==="type"){f.ed=1;RUOLI.setRole(f,x.value)}else f[k]=k==="use"?x.checked:v;
      if(k==="conc")f.gC=false;
      if(k==="type"){if(isCal(f)&&f.conc==null){f.conc=concFrom(f);f.gC=f.conc!=null}if(!isCal(f)){f.conc=null;f.gC=false}}
      // the table is sorted again when a value is committed (change/blur/Enter), never while typing. A change caused by leaving the field (click on another field, Tab) is applied
      // once the focus has moved, and the focus goes back to the field that received it: the rebuilt table must not swallow the click
      const go=()=>{const a=/** @type {HTMLElement} */(document.activeElement),ok=a&&el.contains(a),nk=ok&&a.dataset.k?{name:F[+(a.dataset.i||0)].name,k:a.dataset.k}:ok&&a.dataset.rm?{name:F[+a.dataset.rm].name,rm:1}:keep;sortFiles();renderFiles();
        if(nk){const i=F.findIndex(q=>q.name===nk.name),b=el.querySelector(nk.rm?`[data-rm="${i}"]`:`[data-i="${i}"][data-k="${nk.k}"]`);if(b){b.focus();if(b.type==="text")b.setSelectionRange(b.value.length,b.value.length)}}};
      if(keep)go();else setTimeout(go,0)};
    if(x.dataset.k==="conc")x.addEventListener("keydown",e=>{if(e.key==="Enter"){F[+x.dataset.i].gC=false;x.classList.remove("guess");x.title="";modeNote()}})});
  el.querySelectorAll("[data-dl]").forEach(b=>b.onclick=()=>CONV.download(b.dataset.dl));const dla=$("#dlall");if(dla)dla.onclick=()=>CONV.downloadAll();
  const tuEl=$("#tunit");if(tuEl)tuEl.onchange=()=>{tunitSet(tuEl.value);renderFiles()};
  const cu=$("#cunit");if(cu)cu.onchange=()=>{const k=cu.value,r=CU[ST.unit][1]/CU[k][1];F.forEach(f=>{if(f.conc!=null)f.conc=roundc(f.conc*r)});ST.unit=k;renderFiles()};
  el.querySelectorAll("[data-rm]").forEach(b=>b.onclick=async()=>{try{const f=F[+b.dataset.rm];const j=await post("api/remove",{name:f.name});mergeFiles(j.files)}catch(e){fail(e)}});checkGo();modeNote()}
function modeNote(){const h=$("#hrnote");if(h)h.hidden=!ST.files.some(f=>f.use&&f.hr_profile);const m=$("#modenote");if(!m)return;const u=ST.files.filter(f=>f.use&&["full","ms2"].includes(f.kind)),has=x=>u.some(f=>f.spectrum_mode===x||f.spectrum_mode==="mixed");m.hidden=!(has("profile")&&has("centroid"))}
// the button is live only when at least one chosen file can be opened (an mzML with no data cannot)
const usable=f=>f.use&&["full","ms2","mrm","infusion"].includes(f.kind);
// ...and not while files are still being read and indexed: the button would open only the ones already ready (UPBUSY = loadings under way)
let UPBUSY=0;const LOADFAIL=window.QQQ_LOADFAIL=new Set();      // names of the files that could not be read: mzFinder warns that they are missing from its series
// hook for a progress bar on the loading screen: loadingStato("phase", 40) shows the phase and 40 % in place of the phrase, loadingStato(null) gives the phrases back
function loadingStato(text,pct){ldMsg=text?{text,pct:pct==null?null:pct}:null;ldRefresh()}
// the button waits for the files still being read: its click shows the overlay with the phase and the percentage of the whole batch (event mzlab-index), then opens them all
const LOADP=new Map(),BATCH={tot:0,done:0},IDLE=[];let WAITING=false;
const PHASE_KEY={read:"load.ph.read",scans:"load.ph.scans",chrom:"load.ph.chrom"};
function waitStato(){if(!WAITING)return;let sum=0,low=null;for(const v of LOADP.values()){sum+=v.frac;if(!low||v.frac<low.frac)low=v}
  loadingStato(I18N.t(PHASE_KEY[low&&low.phase]||"load.ph.index"),BATCH.tot?100*Math.min(.99,(BATCH.done+sum)/BATCH.tot):null)}
addEventListener("mzlab-index",e=>{const d=/** @type {CustomEvent} */(e).detail;LOADP.set(d.name,{frac:d.frac,phase:d.phase});waitStato()});
const idle=()=>UPBUSY>0?new Promise(ok=>IDLE.push(ok)):Promise.resolve();
function checkGo(){$("#opbtn").disabled=UPBUSY===0&&!ST.files.some(usable);$("#gomsg").textContent=""}
// sends a file and returns the JSON answer (progress callback: loaded, total); in the browser version there is no network: it goes to the worker
const sendFile=(url,f,onp)=>window.QQQ_BROWSER?fetch(url,{method:"POST",body:f}).then(r=>{onp(1,1);return r.json()}):new Promise((ok,ko)=>{const x=new XMLHttpRequest();x.open("POST",url);
  x.upload.onprogress=e=>{if(e.lengthComputable)onp(e.loaded,e.total)};x.onload=()=>{let j={};try{j=JSON.parse(x.responseText)}catch(_){}ok(j)};x.onerror=()=>ko(new Error(I18N.t("err.upload.network")));x.send(f)});
const DEMO_SETS={full:[0,5,10,15,30].map(t=>`FullScan_t${t}.mzML`),ms2:[15,60].map(t=>`MS2_t${t}.mzML`),mrm:["0.6","2.4","7.2","18"].map(c=>`MRM_std_${c}ppm.mzML`),hrms:[0,5,10,30].map(t=>`HRMS_t${String(t).padStart(3,"0")}.mzML`)};
(()=>{const d=$("#demo");if(!d)return;
 const ch=$("#demochoice"),b=$("#demobtn");
 // the button only opens the choice; a set (Full Scan ~28 MB, MS2 ~10 MB, MRM ~2 MB, HRMS ~28 MB) is downloaded only when it is chosen, and the button says how far it has got
 const setOpen=on=>{ch.hidden=!on;ch.style.display=on?"flex":"none";b.setAttribute("aria-expanded",on?"true":"false")};
 b.onclick=()=>setOpen(ch.hidden);
 ch.querySelectorAll("[data-demo]").forEach(o=>o.onclick=async()=>{const files=DEMO_SETS[o.dataset.demo],lab=b.innerHTML;setOpen(false);b.disabled=true;let got=0;
  const show=i=>{b.textContent=I18N.t("load.demo.progress",{i,n:files.length,mb:I18N.fix(got/1e6,1)})};
  try{const fs=[];for(const [i,n] of files.entries()){show(i+1);const r=await fetch("static/esempi/"+n);if(!r.ok)throw new Error(n);const parts=[];
    if(r.body&&r.body.getReader){const rd=r.body.getReader();for(;;){const {done,value}=await rd.read();if(done)break;parts.push(value);got+=value.length;show(i+1)}}
    else{const bl=await r.blob();parts.push(bl);got+=bl.size}
    fs.push(new File(parts,n))}
   b.textContent=I18N.t("load.demo.opening");await upload(fs,"data")}catch(e){fail(new Error(I18N.t("load.demo.error",{message:e.message})))}finally{b.disabled=false;b.innerHTML=lab}})})();
// files of the instruments are not read by the program (no reader of proprietary formats): a message with the recipe instead of a generic error
// full screen of the whole app (Fullscreen API; the button stays hidden where the browser has none, e.g. iPhone)
(()=>{const b=$("#fs-btn"),de=document.documentElement;if(!b||!de.requestFullscreen)return;b.hidden=false;
  const sync=()=>{const on=!!document.fullscreenElement,k=on?"header.fullscreen.exit":"header.fullscreen";b.setAttribute("aria-pressed",on);b.querySelector(".fs-in").hidden=on;b.querySelector(".fs-out").hidden=!on;b.title=I18N.t(k);b.setAttribute("aria-label",b.title);b.dataset.i18nTitle=b.dataset.i18nAriaLabel=k};
  b.onclick=()=>{(document.fullscreenElement?document.exitFullscreen():de.requestFullscreen({navigationUI:"hide"})).catch(()=>{})};document.addEventListener("fullscreenchange",sync)})();
(()=>{const p=window.MZLAB_RUST_READY;if(p)p.then(b=>{const d=$("#drop");if(b&&d&&b.nativo){d.setAttribute("data-i18n-html","load.dropWiff");d.innerHTML=I18N.t("load.dropWiff");const h=document.querySelector("h3[data-i18n=\"load.step1.title\"]");if(h){h.setAttribute("data-i18n","load.step1.titleNative");h.textContent=I18N.t("load.step1.titleNative")}CONVERT_RAW=true;$("#convhelp").hidden=true
}})})();   // the .raw note only where the Rust engine can read it
// a Thermo .raw is read as it is by the Rust engine (rust-nativo.js): each file is a sample `x.mzML` and no mzML is written; without the engine the recipe is shown
async function rawToMzml(files,box){const rb=window.MZLAB_RUST_READY?await window.MZLAB_RUST_READY:null;if(!rb||rb.disabled||!rb.nativo||!files.some(f=>/\.raw$/i.test(f.name)))return files;
  return NATIVO.raws(files,(ST.files||[]).map(x=>x.name))}
const WIFF_WAIT=new Map();   // stem (lower case) -> {stem,wiff,scan}: a file of a SCIEX pair that came without the other, kept for a later loading
// SCIEX .wiff + .wiff.scan, paired by name (also when they come in different loadings): the Rust engine reads them as they are and each sample (injection) is a file of the list. Gives the files to go on with and the notices [message, name]
async function wiffToMzml(files,box){const rb=window.MZLAB_RUST_READY?await window.MZLAB_RUST_READY:null,res=/** @type {any} */ ({files,errs:[]}),isW=f=>/\.wiff(\.scan)?$/i.test(f.name);
  if((rb?(rb.disabled||!rb.nativeSamples):window.MZLAB_RUST_ERR&&!window.QQQ_PHONE)&&files.some(isW)){      // the engine exists but cannot convert now (it stopped twice in a minute, or the page and its scripts are of two versions): say so, not «cannot be opened in the browser»
    res.files=files.filter(f=>!isW(f));files.filter(isW).forEach(f=>res.errs.push([I18N.t(rb?"err.file.wiffEngine":"err.file.wiffLoad"),f.name]));return res}
  if(!rb||rb.disabled||!rb.nativeSamples||!files.some(isW))return res;
  res.files=files.filter(f=>!isW(f));const T=(k,p)=>I18N.t(k,p),dropped=new Set();
  for(const f of files.filter(isW)){const sc=/\.scan$/i.test(f.name),stem=f.name.replace(/\.wiff(\.scan)?$/i,""),k=CartCarica.pairKey(f,stem),e=WIFF_WAIT.get(k)||{stem};e[sc?"scan":"wiff"]=f;WIFF_WAIT.set(k,e);dropped.add(k)}
  const taken=[...res.files.map(x=>x.name),...(ST.files||[]).map(x=>x.name)],pairs=[];
  for(const [k,e] of [...WIFF_WAIT]){      // a pair is in the list as soon as its two files are together, before anything is read
    if(!(e.wiff&&e.scan)){if(dropped.has(k)){if(e.wiff)(res.lone=res.lone||[]).push({e,rb});else res.errs.push([T("load.wiffWait",{name:e.scan.name}),e.scan.name])}continue}      // a .wiff alone: after the loading it may turn out to be the method of an mzML (loneWiffs)
    WIFF_WAIT.delete(k);e.row=e.stem+".wiff";pairs.push(e);CONVROWS.add(e.row,"open")}
  for(const e of pairs){      // each sample (injection) is a file of the list: the .wiff and the .wiff.scan are read as they are
    try{const mem=[{name:e.wiff.name,blob:e.wiff},{name:e.scan.name,blob:e.scan}],M0=window.LETTORI_MOD,pr=M0&&window.LETTORI_FAM.sciex.sonda?await M0.probe("sciex",mem).catch(()=>null):null;      // the module looks at every pair first: the engine reads the 3200 QTRAP only, and a file of another instrument or scan type is for the module («variante»)
      if(pr&&pr.motivi.includes("variante")){const cs=pr.campioni.length?pr.campioni:[{id:1}];cs.forEach((x,i)=>{const nm=wiffName(e.stem,x.id,cs.length>1,taken);taken.push(nm);res.files.push(M0.stand(nm,mem,i,"sciex"))});CONVROWS.drop(e.row);continue}
      const s=(await rb.nativeSamples(mem)).samples,key=NATIVO.key(e.stem);
      s.forEach((x,i)=>{const nm=wiffName(e.stem,x.id,s.length>1,taken);taken.push(nm);res.files.push(NATIVO.sample(nm,key,mem,i,x.id,false))})}
    catch(x){const M=window.LETTORI_MOD,fam=M&&x.json?M.retriable(x.json.error||x.json.error_key):"";if(fam)res.files.push(M.stand(wiffName(e.stem,1,false,taken),[{name:e.wiff.name,blob:e.wiff},{name:e.scan.name,blob:e.scan}],0,fam));else res.errs.push([x.json?I18N.err(x.json).message:x.message,e.wiff.name])}
    CONVROWS.drop(e.row)}
  return res}
// Rule R (docs/agenti/lettori-modulo.md): the readers module (lettori-modulo.js) writes the mzML of what the engine does not read, into OPFS, and the mzML enters upload() as any other: a file with an extension of the
// module, a sample the engine refused with an error of variant (File with `mzlabModulo`, made by LETTORI_MOD.stand), an mzML with numpress arrays, a dataset the module's probe says the engine cannot do, and, with ?lettori=modulo, a .D folder (proof of the path).
const LETT_FORCE=/[?&]lettori=modulo\b/.test(location.search);
/** the family whose module takes `f` ("" = none): a sample the engine refused, a file with an extension only a module reads, a dataset the module of its family says the engine cannot do (probe) */
async function moduloWants(M,f){
  if(f.mzlabModulo)return f.mzlabModulo.fam;const n=f.name.toLowerCase();
  if(M.extensions().some(e=>n.endsWith(e)))return M.famigliaDi([n]);
  if(n.endsWith(".mzml")&&!f.mzlabNat){const v=await M.variante(f);if(v)return v}      // an mzML with numpress arrays: the engine refuses it (err.file.numpress), the module writes it again with zlib
  const nat=f.mzlabNat;if(!nat)return "";const fam=M.famigliaDi(nat.members.map(m=>m.name));if(!fam)return "";
  if(LETT_FORCE&&nat.members.some(m=>/\.d\//i.test(m.name)))return fam;
  if(!M.loaded)return "";
  try{return (await M.probe(fam,nat.members)).motivi.length>0?fam:""}catch(_){return ""}}
async function moduloToMzml(files,box,rep){
  const M=window.LETTORI_MOD;if(!M||!files.length)return files;
  const jobs=new Map();for(const f of files){const fam=await moduloWants(M,f);if(fam)jobs.set(f,fam)}if(!jobs.size)return files;
  const g=ConvUI.group(box,[...jobs.keys()].map(f=>NATIVO.size(f))),out=/** @type {any} */ ([]),taken=[...(ST.files||[]).map(x=>x.name),...files.filter(f=>!jobs.has(f)).map(f=>f.name)];
  let i=0;
  for(const f of files){
    if(!jobs.has(f)){out.push(f);continue}
    i++;const lot=f.mzlabModulo||f.mzlabNat,mem=lot?lot.members:[{name:f.name,blob:f}],nm=f.mzlabModulo||f.mzlabNat?f.name:M.mzmlName(f.name,taken);taken.push(nm);
    if(g.signal.aborted)continue;
    CONVROWS.add(nm,"open");const ui=g.start(i,nm),dest=`${Date.now().toString(36)}-${Math.random().toString(36).slice(2,8)}.mzML`;
    try{const blob=await M.toMzml(jobs.get(f),mem,lot?lot.c:0,dest,p=>{ui.update(p);CONVROWS.set(nm,"run",p.scansTotal?Math.floor(100*p.scans/p.scansTotal):0)},g.signal);
      ui.done();CONV.add(nm,blob);out.push(new File([blob],nm))}
    catch(e){ui.done();if(!e.cancelled){const m=e.json?I18N.err(e.json).message:e.message;(rep.errs[m]=rep.errs[m]||[]).push(f.name)}}
    finally{CONVROWS.drop(nm)}}
  out.panel=g;return out}
let CONVERT_RAW=false;   // true where the engine converts .raw itself (desktop, ?motore=rust): the guide then appears only for the formats it cannot convert
const UNOPEN_RE=/\.(raw|wiff2?|wiff\.scan|d|lcdx)$/i;
function fileKind(name){const n=name.toLowerCase();if(/\.(mzml|dam)$/.test(n))return null;const M=window.LETTORI_MOD;if(M&&M.extensions().some(e=>n.endsWith(e)))return null;const m=UNOPEN_RE.exec(n);return m?m[0]:"other"}
function ssGet(k){try{return sessionStorage.getItem(k)}catch(e){return null}}
function ssSet(k,v){try{sessionStorage.setItem(k,v)}catch(e){}}
const SEEN_GROUPS=new Set();
function groupHash(files){const s=files.map(f=>f.name+":"+f.size).sort().join("|");let h=5381;for(let i=0;i<s.length;i++)h=(h*33^s.charCodeAt(i))>>>0;return h+"."+files.length}
function groupSeen(h){let a=[];try{a=JSON.parse(ssGet("qqq.unopen")||"[]")}catch(e){}return SEEN_GROUPS.has(h)||a.includes(h)}
function groupMark(h){SEEN_GROUPS.add(h);try{const a=JSON.parse(ssGet("qqq.unopen")||"[]");a.push(h);ssSet("qqq.unopen",JSON.stringify(a.slice(-50)))}catch(e){}}
function namesCut(a,max){return a.slice(0,max).map(esc).join(", ")+(a.length>max?"…":"")}
// ONE notice for the whole loading: files that cannot be opened in the browser (count + collapsible list), unknown formats, read errors and big files, one line per kind
function closeUnopen(){const u=$("#unopen");u.hidden=true;u.textContent=""}
function openConvHelp(focus){const d=$("#convhelp");d.hidden=false;d.open=true;if(focus){d.scrollIntoView({block:"nearest"});d.querySelector("summary").focus()}}
function reportLoad(rep){
  const u=$("#unopen"),L=[],T=(k,p)=>I18N.t(k,p);let conv=[];
  if(rep.unopen.length){
    const by={};rep.unopen.forEach(x=>{by[x.ext]=(by[x.ext]||0)+1;conv.push(x.ext)});
    const types=Object.keys(by).join(", "),n=rep.unopen.length,names=rep.unopen.map(x=>x.name);
    L.push(`<p><b>${T(n===1?"unopen.head.one":"unopen.head",{n,types})}</b></p>`);
    L.push(`<details class="unopen-names"><summary>${T("unopen.names")}</summary><ul>${names.slice(0,5).map(x=>`<li>${esc(x)}</li>`).join("")}${names.length>5?`<li class="muted">${T("unopen.more",{n:names.length-5})}</li>`:""}</ul></details>`)}
  if(rep.other.length)L.push(`<p>${T("unopen.other",{n:rep.other.length,names:namesCut(rep.other,3)})}</p>`);
  for(const [msg,names] of Object.entries(rep.errs))L.push(`<p class="fail">${names.length===1?T("unopen.err.one",{name:esc(names[0]),message:esc(msg)}):T("unopen.err.many",{n:names.length,names:namesCut(names,3),message:esc(msg)})}</p>`);
  if(rep.moved.dam.length)L.push(`<p>${T("load.note.movedToDam",{name:namesCut(rep.moved.dam,3)})}</p>`);
  if(rep.moved.data.length)L.push(`<p>${T("load.note.movedToData",{name:namesCut(rep.moved.data,3)})}</p>`);
  for(const n of rep.notes||[])L.push(`<p>${esc(n)}</p>`);
  if(rep.big.length)L.push(`<p>${T("load.note.big",{n:rep.big.length})}</p>`);
  if(!L.length)return;
  const needHelp=rep.unopen.length||rep.other.length;
  u.innerHTML=L.join("")+`<div class="row">${needHelp?`<button type="button" class="sec" data-a="help">${T("unopen.how")}</button>`:""}<button type="button" class="sec" data-a="close">${T("common.close")}</button></div>`;
  u.hidden=false;
  u.querySelector('[data-a=close]').onclick=closeUnopen;
  const hb=u.querySelector('[data-a=help]');if(hb)hb.onclick=()=>openConvHelp(true);
  if(rep.unopen.length){
    const h=$("#convhint");h.textContent=T("conv.help.hint",{types:[...new Set(conv)].join(", ")});h.hidden=false;
    if(!ssGet("qqq.convopen")){ssSet("qqq.convopen","1");openConvHelp(false)}   // opens by itself once per session
    if(CONVERT_RAW)$("#convdesk").hidden=false}
  else if(needHelp&&CONVERT_RAW)$("#convhelp").hidden=false}
document.addEventListener("keydown",e=>{if(e.key==="Escape"&&!$("#unopen").hidden&&$("#unopen").contains(document.activeElement)){e.preventDefault();closeUnopen()}});
async function loneWiffs(wr,rep){      // a .wiff without its .scan is the method of the mzML files made from it (same SHA-1): else it waits for its .scan
  for(const {e,rb} of wr.lone||[]){try{const r=await rb.metodoWiff(e.wiff);if(r.matched.length){WIFF_WAIT.delete(e.stem.toLowerCase());rep.notes.push(I18N.t("load.note.wiffMethod",{name:e.wiff.name,n:r.matched.length,files:namesCut(r.matched.map(x=>x.name),3)}));continue}}catch(_){}
    const m=I18N.t("err.file.wiffNoScan",{name:e.wiff.name});(rep.errs[m]=rep.errs[m]||[]).push(e.wiff.name)}}
const INFLIGHT=new Set(),fkey=f=>f.name+"|"+f.size;
// the same file (name and size) already in the list, or being read now, is not loaded again: a short notice and its row lights up for a moment
function dropDups(files){const dups=files.filter(f=>INFLIGHT.has(fkey(f))||ST.files.some(g=>g.name===f.name&&g.size===f.size));if(!dups.length)return files;
  const msg=dups.length===1?I18N.t("load.dup.one",{name:dups[0].name}):I18N.t("load.dup.many",{n:dups.length});CartCarica.note(msg);setTimeout(()=>{const n=$("#cartinfo");if(n&&n.textContent===msg)CartCarica.note("")},5000);
  dups.forEach(f=>{const tr=[...document.querySelectorAll("#flist tr")].find(r=>r.children[1]&&r.children[1].textContent.trim().startsWith(f.name));if(tr){tr.classList.add("dupflash");tr.scrollIntoView({block:"nearest"});setTimeout(()=>tr.classList.remove("dupflash"),1800)}});
  return files.filter(f=>!dups.includes(f))}
async function upload(files,into){const box=$("#up");fail();files=dropDups(files);if(!files.length)return;files.forEach(f=>INFLIGHT.add(fkey(f)));if(!UPBUSY){BATCH.tot=0;BATCH.done=0}BATCH.tot+=files.length;UPBUSY++;checkGo();const mine=files;try{await upload0(files,into,box)}finally{mine.forEach(f=>INFLIGHT.delete(fkey(f)));UPBUSY--;checkGo();if(!UPBUSY)IDLE.splice(0).forEach(ok=>ok())}}
async function upload0(files,into,box){
  const rep=/** @type {any} */ ({unopen:[],other:[],errs:{},moved:{dam:[],data:[]},big:[],notes:[]}),rejected=/** @type {any[]} */ ([]);
  const one=async f=>{const isDam=isMethodFile(f.name);
    if(into&&isDam!==(into==="dam"))rep.moved[isDam?"dam":"data"].push(f.name);
    const pn=panel&&CONV.has(f.name)?panel:null,row=document.createElement("div");      // a converted file: its index is the last phase of the bar of the conversion
    if(!pn){row.innerHTML=`${esc(f.name)} <span class="muted">${sz(NATIVO.size(f))}</span><div class="prog"><i></i></div>`;box.appendChild(row)}
    CONVROWS.set(f.name,"index",0);if(pn)pn.index(f.name,0);
    const onIdx=e=>{if(e.detail.name===f.name){const pc=Math.min(99,Math.floor(100*e.detail.frac));CONVROWS.set(f.name,NATIVO.rowState(e.detail.phase),pc);if(pn)pn.index(f.name,pc);else row.querySelector("i").style.width=pc+"%"}};
    addEventListener("mzlab-index",onIdx);
    try{const j=await sendFile("api/upload?name="+encodeURIComponent(f.name),f,(a,b)=>{if(!pn)row.querySelector("i").style.width=(100*a/b)+"%";CONVROWS.set(f.name,"index",Math.floor(100*a/b));if(pn)pn.index(f.name,Math.floor(100*a/b))});
      if(j.error)throw I18N.err(j);CONVROWS.drop(f.name);if(pn)pn.index(f.name,100);mergeFiles(j.files);ST.methods=j.methods||ST.methods;renderMethods();row.remove();
      LOADFAIL.delete(f.name);if(window.QQQ_BROWSER&&NATIVO.size(f)>300*1048576)rep.big.push(f.name)}
    catch(e){row.remove();CONVROWS.drop(f.name);LOADFAIL.add(f.name);(rep.errs[e.message]=rep.errs[e.message]||[]).push(f.name)}finally{removeEventListener("mzlab-index",onIdx);LOADP.delete(f.name);BATCH.done++;waitStato()}};
  let panel=null;
  const wr=await wiffToMzml(files,box);files=await rawToMzml(wr.files,box);files=await moduloToMzml(files,box,rep);panel=files.panel||null;for(const [m,n] of wr.errs)(rep.errs[m]=rep.errs[m]||[]).push(n);
  files=files.filter(f=>{const k=fileKind(f.name);if(!k)return true;rejected.push(f);if(k==="other")rep.other.push(f.name);else rep.unopen.push({name:f.name,ext:k});return false});
  if(rep.unopen.length){const h=groupHash(rep.unopen.map(x=>rejected.find(f=>f.name===x.name)));if(groupSeen(h)){rep.unopen=[];rep.other=[]}else groupMark(h)}   // the same group dropped again: no second notice
  if(window.QQQ_BROWSER&&files.length>1)await Promise.all(files.map(one));else for(const f of files)await one(f);   // in the browser the engine builds the indexes of several files at once (pool of workers)
  await loneWiffs(wr,rep);if(panel)panel.end();reportLoad(rep)}
function showStart(d){mergeFiles(d.files||[]);ST.methods=d.methods||[];renderMethods();applyView()}
$("#dropdam").onclick=()=>$("#pickdam").click();$("#dropdam").onkeydown=e=>{if(e.key==="Enter"||e.key===" "){e.preventDefault();$("#pickdam").click()}};$("#pickdam").onchange=e=>{upload([...e.target.files],"dam");e.target.value=""};
$("#cartpick").onclick=()=>{if(typeof CartPunt!=="undefined"&&(CartPP.supported()||CartPP.S.native))CartPunt.choose();else $("#pickdir").click()};$("#pickdir").onchange=e=>{CartCarica.fromInput(e.target.files);e.target.value=""};
$("#drop").onclick=()=>$("#pick").click();$("#pick").onchange=e=>{upload([...e.target.files],"data");e.target.value=""};
["dragenter","dragover"].forEach(t=>$("#drop").addEventListener(t,e=>{e.preventDefault();$("#drop").classList.add("over")}));
["dragleave","drop"].forEach(t=>$("#drop").addEventListener(t,e=>{e.preventDefault();$("#drop").classList.remove("over")}));
$("#drop").addEventListener("drop",e=>{if(!CartCarica.fromDrop(e))upload([...e.dataTransfer.files],"data")});   // a dropped folder is read by cartella-carica.js
["dragenter","dragover"].forEach(t=>$("#dropdam").addEventListener(t,e=>{e.preventDefault();$("#dropdam").classList.add("over")}));
["dragleave","drop"].forEach(t=>$("#dropdam").addEventListener(t,e=>{e.preventDefault();$("#dropdam").classList.remove("over")}));
$("#dropdam").addEventListener("drop",e=>upload([...e.dataTransfer.files],"dam"));
addEventListener("dragover",e=>e.preventDefault());addEventListener("drop",e=>e.preventDefault());   // a missed drop must not open the file
// The list is opened file by file: the first one at once (the view opens on it), the others as they are read, in the order of the list. The files still to come
// are in E.pending (grey rows, aria-busy, not selectable); a file that fails stays there in red with its message and the others go on; a file taken off the list is not read.
async function exploreList(samples){
  if(samples.length<2||!(window.QQQ_BROWSER||/[?&]progressivo=1\b/.test(location.search))){await post("api/explore",{samples});return null}
  await post("api/explore",{samples:[samples[0]],all:samples.map(s=>s.file)});
  const tok=E.progTok={};E.pendN=samples.length;
  E.pending=samples.slice(1).map(s=>({file:s.file,label:s.label||s.file,state:"wait"}));
  return ()=>openRest(samples.slice(1),tok)}
async function openRest(rest,tok){
  let last=0,added=[];
  const boot=async()=>{last=Date.now();const a=added;added=[];await bootSession();E.pending=E.pending.filter(x=>!a.includes(x));renderFileList()};
  for(const s of rest){
    if(E.progTok!==tok)return;
    const p=E.pending.find(x=>x.file===s.file&&x.state==="wait");if(!p)continue;          // taken off the list: not read
    try{await post("api/explore",{samples:[s],append:true})}
    catch(e){p.state="err";p.msg=e.message;renderFileList();continue}
    added.push(p);if(Date.now()-last>=200)await boot();               // LR: the overlay is redrawn at most every ~200 ms, whatever the number of files that arrived
  }
  if(E.progTok===tok){if(added.length)await boot();E.pendN=0;renderFileList();uiSave()}}
$("#opbtn").onclick=async()=>{const b=$("#opbtn");b.disabled=true;loading(true);
  try{if(UPBUSY>0){WAITING=true;waitStato();await idle();WAITING=false;loadingStato(null)}
    if(!ST.files.some(usable)){checkGo();return}
    ST.files.forEach(f=>{if(RUOLI.roleOf(f)==="dark")f.time=darkTime(f.time)});      // the time of a dark is negative whatever way it got here (name, table, old project)
    const rest=await exploreList(ST.files.filter(usable).map(f=>({file:f.name,time:f.time,type:f.type,conc:isCal(f)?f.conc:null,cunit:CU[ST.unit][0],label:(E.files.find(x=>x.file===f.name)||{}).label})));STATO.aggiunta(false,"adding");await bootSession();RUOLI.commit(ST.files);applyView();window.scrollTo(0,0);await Promise.all(E.panels.map(p=>p.ready).filter(Boolean)).catch(()=>{});uiSave();if(rest)rest()}
  catch(e){
    if(e.key==="err.hr.mix"){
      $("#gomsg").innerHTML=`<span class="fail">${I18N.t("err.hr.mix")}</span> 
      <button id="hrmix-replace" class="btn">${I18N.t("load.hrmix.replace")}</button> 
      <button id="hrmix-cancel" class="btn">${I18N.t("common.cancel")}</button>`;
      loading(false);
      const isOpen=f=>typeof E!=="undefined"&&E.files.some(x=>x.file===f.name||x.file.startsWith(f.name+"#"));
      $("#hrmix-cancel").onclick=()=>$("#cancadd").click();
      $("#hrmix-replace").onclick=async()=>{
        const b=$("#hrmix-replace");b.disabled=true;loading(true);
        try {
          await post("api/new",{});                                 // closes the open files but keeps the uploaded ones on disk
          await post("api/explore",{samples:ST.files.filter(f=>usable(f)&&!isOpen(f)).map(f=>({file:f.name,time:f.time,type:f.type,conc:isCal(f)?f.conc:null,cunit:CU[ST.unit][0],label:null}))});
          STATO.aggiunta(false,"adding");await bootSession();RUOLI.commit(ST.files);applyView();await Promise.all(E.panels.map(p=>p.ready).filter(Boolean)).catch(()=>{});uiSave();
        }catch(e2){$("#gomsg").innerHTML=h`<span class="fail">${e2.message}</span>`;b.disabled=false}finally{loading(false)}
      };
    } else {
      $("#gomsg").innerHTML=h`<span class="fail">${e.message}</span>`;
    }
    b.disabled=false
  }finally{if($("#gomsg").innerHTML==="")loading(false)}
};
$("#cancadd").onclick=()=>{STATO.aggiunta(false,"adding");applyView()};
// while adding files the ones already in the session stay visible, with a way back to the data
function renderLoaded(){const el=$("#loaded"),on=!!(S.sess&&S.adding&&typeof E!=="undefined"&&E.files.some(f=>!f.gone));el.hidden=!on;if(!on)return;
  const fs=E.files.filter(f=>!f.gone);
  el.innerHTML=`<div class="row" style="display:flex;gap:10px;align-items:center;flex-wrap:wrap"><b>${I18N.t("load.loaded.title",{n:fs.length})}</b><span class="sp" style="flex:1"></span><button class="go" id="backdata">${I18N.t("load.loaded.back")}</button></div><div class="muted sm" style="margin-top:6px">${fs.map(f=>`<span style="white-space:nowrap;margin-right:12px"><span style="display:inline-block;width:9px;height:9px;border-radius:2px;background:${f.color||"#888"};margin-right:4px"></span>${esc(f.label||f.file)}</span>`).join("")}</div>`;
  $("#backdata").onclick=()=>{STATO.aggiunta(false,"adding");applyView()}}
// New session: QQRESET.ask (reset.js) asks once, then clears everything the app keeps in this browser and reloads
$("#newrun").onclick=()=>QQRESET.ask();
// "m/z" is always written in italics: any text node that contains it is split and the symbol wrapped in <i> (not inside inputs, options, scripts or an existing <i>)
(function(){const SKIP=/^(SCRIPT|STYLE|OPTION|TEXTAREA|INPUT|CANVAS|TITLE|SELECT)$/,RX=/(m\/z|MS2)/;
 // m/z always in italics and MS2 with a real superscript (never the Unicode characters); text nodes only: titles, options and file names keep the plain "MS2"
 function fix(root){const w=document.createTreeWalker(root,NodeFilter.SHOW_TEXT,{acceptNode:n=>RX.test(/** @type {string} */ (n.nodeValue))&&n.parentElement&&!SKIP.test(n.parentElement.tagName)&&!(n.parentElement.tagName==="I"&&n.nodeValue==="m/z")&&!(n.parentElement.tagName==="SUP")?NodeFilter.FILTER_ACCEPT:NodeFilter.FILTER_REJECT});
  const L=[];while(w.nextNode())L.push(w.currentNode);
  for(const n of /** @type {any[]} */ (L)){const f=document.createDocumentFragment();n.nodeValue.split(/(m\/z|MS2)/).forEach(t=>{if(t==="m/z"){const e=document.createElement("i");e.textContent="m/z";f.appendChild(e)}else if(t==="MS2"){f.appendChild(document.createTextNode("MS"));const e=document.createElement("sup");e.textContent="2";f.appendChild(e)}else if(t)f.appendChild(document.createTextNode(t))});n.replaceWith(f)}}
 let busy=false;new MutationObserver(ms=>{if(busy)return;busy=true;try{for(const m of ms){if(m.type==="characterData"){if(m.target.parentElement)fix(m.target.parentElement)}else m.addedNodes.forEach(a=>{if(a.nodeType===1)fix(a);else if(a.nodeType===3&&a.parentElement)fix(a.parentElement)})}}finally{busy=false}}).observe(document.body,{childList:true,subtree:true,characterData:true});
 fix(document.body)})();
document.querySelectorAll("[data-ic]").forEach(e=>{e.innerHTML=QICON.get(e.dataset.ic,16);
  if(e.closest("header"))e.querySelectorAll("svg").forEach(v=>{v.removeAttribute("aria-label");v.removeAttribute("role");v.setAttribute("aria-hidden","true")})});   // the header icons sit next to their text label (the icon names are Italian)
// What is downloaded when (decided 8/10/2026, measured on the site): the engine (~10 MB compressed) at once, because Dati needs it;
// after it, one step per idle moment: OpenChemLib ~340 kB, then (fast connection) Ketcher, then the Teoria ~60 kB; Ketcher (~8.7 MB) only when the mouse goes on «Disegno», or when idle if the connection says it is fast
// and not metered (Chrome: navigator.connection, downlink >= 10 Mbit/s, no data saver); the example files (~28 MB) only with their button.
function prefetch(urls,as){urls.forEach(u=>{const l=document.createElement("link");l.rel=as==="module"?"modulepreload":"prefetch";l.href=u;if(as&&as!=="module")l.as=as;document.head.appendChild(l)})}
let kPre=false;
function prefetchKetcher(){if(kPre||window.QQQ_PHONE)return;kPre=true;
  fetch("static/vendor/ketcher/index.html").then(r=>r.text()).then(t=>{const u=[...t.matchAll(/(?:src|href)="\.?\/?(assets\/[^"]+)"/g)].map(m=>"static/vendor/ketcher/"+m[1]);
    u.forEach(x=>fetch(x,{priority:"low"}).then(r=>r.arrayBuffer()).catch(()=>{}))}).catch(()=>{kPre=false})}
function prefetchIdle(){const idle=/** @type {any} */ (window).requestIdleCallback;
  const c=/** @type {any} */ (navigator).connection||{};if(c.saveData)return;
  const fast=c.downlink>=10&&!/2g|3g/.test(c.effectiveType||"");
  // one step per idle moment, in this order: Disegno (OpenChemLib, Ketcher if fast), then the Teoria (the Dati scripts are all in carica.js's list; the other language is fetched only when chosen)
  const steps=[()=>prefetch(["static/vendor/openchemlib.js"],"module"),()=>{if(fast)prefetchKetcher()},
    ()=>prefetch(["static/teoria/index.html","static/teoria/teoria.css","static/teoria/teoria.js","static/teoria/tema.js","static/teoria/glossario-dati.js"])];
  const next=()=>{const f=steps.shift();if(!f)return;f();idle?idle(next,{timeout:8000}):setTimeout(next,2000)};
  idle?idle(next,{timeout:8000}):setTimeout(next,2000)}
{const b=document.querySelector('#nav button[data-v="draw"]');if(b){b.addEventListener("pointerenter",prefetchKetcher,{once:true});b.addEventListener("focus",prefetchKetcher,{once:true})}}
{const s=document.createElement("script");s.type="module";s.src="static/draw.js";document.body.appendChild(s);init()}   // draw.js (Disegno, with OpenChemLib) is not even fetched on a phone
// Five logo hits turn the administrator mode on without a reload: the Teoria already open in the frame reloads, so its editor button and Alt+E appear
document.addEventListener("mzlab:admin",()=>{const f=$("#tframe");if(f&&f.getAttribute("src")&&f.contentWindow)try{f.contentWindow.location.reload()}catch(_){}});
