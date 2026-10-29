// @ts-strict
// @ts-noany
// Page shell: helpers shared with explore.js (classic scripts share globals), start screen, views.
const $=(/** @type {string} */ s)=>document.querySelector(s);
// addresses of the views: #/data, #/draw (and #/theory, which the head script above sends to the standalone Teoria on a cold start)
const fmt=(/** @type {number} */ v)=>v===0?"0":(Math.abs(v)>=1e4||Math.abs(v)<.01)?v.toExponential(1):(+v.toPrecision(3)).toString();
const css=(/** @type {string} */ n)=>getComputedStyle(document.documentElement).getPropertyValue(n).trim();
const esc=(/** @type {unknown} */ t)=>String(t).replace(/[&<>"]/g,c=>(/** @type {Record<string, string>} */ ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}))[c]);
// fmtFormula / fmtAdduct (subscripts, superscripts) are defined in explore.js, loaded before this script
async function api(/** @type {string} */ u){const r=await fetch(u);const j=await r.json();if(j.error)throw I18N.err(j);return j}
// an error of any shape (Error, answer of api/, text) or nothing
/** @param {unknown} [e_] */ function fail(e_){const e=/** @type {{ message?: string } | undefined} */(e_);$("#err").textContent=e?String(e.message||e):""}
// canvas resolution = device pixels x the pinch scale (a "visual" zoom on a Mac trackpad), at most 4; every canvas is redrawn when it changes
function dprNow(){const s=(window.visualViewport&&window.visualViewport.scale)||1;return Math.min(4,(devicePixelRatio||1)*s)}
{let last=dprNow(),t=0;const redo=()=>{const d=dprNow();if(Math.abs(d-last)<.01)return;last=d;if(typeof redrawAll==="function")redrawAll()};
 // browser zoom (Ctrl/Cmd +)
 const arm=()=>{try{matchMedia(`(resolution: ${devicePixelRatio}dppx)`).addEventListener("change",()=>{redo();arm()},{once:true})}catch(_){}};arm();
 addEventListener("resize",()=>{clearTimeout(t);t=setTimeout(redo,60)});
 // pinch: after a short pause
 if(window.visualViewport)window.visualViewport.addEventListener("resize",()=>{clearTimeout(t);t=setTimeout(redo,150)})}
// layout size, not getBoundingClientRect: a panel sliding in (transizioni.js scales it) must not change the resolution
function setup(/** @type {HTMLCanvasElement} */ c){const z=typeof fz==="function"?fz():1;M.l=Math.max(Math.round(66*z),window.MARG_L||0);M.r=Math.max(14,window.MARG_R||0);M.t=Math.round(14*z);M.b=Math.round(40*z);const r={width:c.clientWidth,height:c.clientHeight},d=dprNow();c.width=Math.round(r.width*d);c.height=Math.round(r.height*d);const g=/** @type {CanvasRenderingContext2D} */ (c.getContext("2d"));g.setTransform(d,0,0,d,0,0);return{g,W:r.width,H:r.height}}
function nice(/** @type {number} */ a,/** @type {number} */ b,/** @type {number} */ n){const s=(b-a)/n,p=Math.pow(10,Math.floor(Math.log10(s))),m=s/p,st=(m<1.5?1:m<3?2:m<7?5:10)*p,o=[];for(let t=Math.ceil(a/st)*st;t<=b+1e-9;t+=st)o.push(+t.toFixed(10));return o}
function smooth(/** @type {number[]} */ y,/** @type {number} */ k){if(!k)return y;const n=y.length,o=new Array(n);for(let i=0;i<n;i++){const v=y[i],a=i?y[i-1]??v:v,c=i<n-1?y[i+1]??v:v;o[i]=(a+2*v+c)/4}return o}
const M={l:66,r:14,t:14,b:40};   // plot margins (px): room for tick labels and axis titles

async function init(){try{await bootSession();await Promise.all(E.panels.map((/** @type {{ ready?: unknown }} */ p)=>p.ready).filter(Boolean)).catch(()=>{});
  const d=await api("api/state");fail();window.qqReady=true;if(typeof AVVIO!=="undefined")AVVIO.segnaPronto();if(window.ldClearBootTimer)window.ldClearBootTimer();setTimeout(prefetchIdle,3000);const hasSess=!!(d.app||S.sess);if(typeof AVVIO!=="undefined")AVVIO.segnaSessione(!!S.sess);$("#newrun").hidden=!hasSess;if(!S.sess)showStart(d);else{applyView();if(window.QQRESET)QQRESET.remind()}if(window.BARRA)BARRA.init()}catch(e){fail(e);if(typeof AVVIO!=="undefined")AVVIO.segnaFallito(e)}finally{if(!$("#loading").hidden)loading(false)}}

// ------------------------------------------------------------------ views: Dati (start screen or panels), Disegno
let _lastV="",_itNoted=false;
// true only the first time: remembered in localStorage (the reset of the settings of the Teoria clears it), per session when there is no storage
function itNoteOnce(){const k="mzlab.teoria.itnote";try{if(localStorage.getItem(k))return false;localStorage.setItem(k,"1");return true}catch(e){const r=!_itNoted;_itNoted=true;return r}}
let _addY=/** @type {number | null} */ (null),_wasAdd=false;
function applyView(){const v=S.view,d=v==="data";
  // the scroll of the data view, kept for the way back from the loading screen (leaveAdding)
  if(S.adding&&!_wasAdd)_addY=window.scrollY;_wasAdd=!!S.adding;
  $("#start").hidden=!(d&&(!S.sess||S.adding));$("#v-data").hidden=!(d&&S.sess&&!S.adding);$("#cancadd").hidden=!(S.sess&&S.adding);renderLoaded();
  if(window.BARRA){if(d&&S.sess&&!S.adding)BARRA.dockCalc(true);else BARRA.dockCalc(false);}
  $("#v-draw").hidden=v!=="draw";document.documentElement.classList.toggle("in-draw",v==="draw");$("#v-theory").hidden=v!=="theory";
  // The Teoria is Italian only: in English it opens at once in Italian, with a short toast the first time the student enters it (once per session without storage)
  // theory module, loaded on first open
  if(v==="theory"&&!$("#tframe").getAttribute("src"))$("#tframe").src="static/teoria/index.html";
  if(v==="theory"&&_lastV!=="theory"&&I18N.lang==="en"&&window.toast&&itNoteOnce())toast(I18N.t("theory.onlyItalian"));
  _lastV=v;
  document.querySelectorAll("#nav button").forEach(b=>b.classList.toggle("on",b.dataset.v===v));
  // one title per view (WCAG 2.4.2)
  // tool tabs (Flow, Tabui, ...) carry their own name on the nav button
  { const tb=document.querySelector('#nav button[data-v="'+v+'"]'),tn=tb&&tb.dataset.tname,T=(v==="data"?I18N.t("app.title",{app:window.APP_NAME||""}):(tn||I18N.t("nav."+(v==="draw"?"draw":"theory")))+" · "+(window.APP_NAME||"")); if(document.title!==T)document.title=T; }
  document.dispatchEvent(new CustomEvent("tpview",{detail:{view:v}}))}
// the header stays in view (sticky): --hh is its real height (it can wrap on a narrow window), used by the file list, the full-screen panel and the scrolls to a panel
{const hd=document.querySelector("header"),hh=()=>{document.documentElement.style.setProperty("--hh",hd.offsetHeight+"px");hd.classList.toggle("scrolled",scrollY>4)};
 hh();addEventListener("scroll",()=>hd.classList.toggle("scrolled",scrollY>4),{passive:true});addEventListener("resize",hh);if(window.ResizeObserver)new ResizeObserver(hh).observe(hd)}
window.hdrH=()=>document.querySelector("header").offsetHeight;
// Back and Forward walk through the views
/** @param {boolean} [fromHistory] */ function setView(/** @type {string} */ v,fromHistory){v=VIEW_ALIAS[v]||v;STATO.vista(v,"view");applyView();if(!fromHistory&&VIEW_HASH[v]&&location.hash!=="#/"+VIEW_HASH[v])history.pushState(null,"","#/"+VIEW_HASH[v]);}
document.querySelectorAll("#nav button").forEach(b=>b.onclick=()=>setView(b.dataset.v));
// Back/Forward and typed addresses: the view follows the hash (a cold #/theory never gets here: the head script leaves the page)
addEventListener("popstate",()=>{const v=hashView()||"data";if(v!==S.view)setView(v,true)});
// a direct address shows its view at once, without waiting for the engine (the loading veil stays until the program is ready)
if(S.view!=="data"){$("#v-draw").hidden=S.view!=="draw";$("#v-theory").hidden=S.view!=="theory";document.documentElement.classList.toggle("in-draw",S.view==="draw");document.querySelectorAll("#nav button").forEach(b=>b.classList.toggle("on",b.dataset.v===S.view))}

// ------------------------------------------------------------------ start screen: upload, check times and types, open
const sz=(/** @type {number | null | undefined} */ b)=>b==null?"—":b>1e6?(b/1e6).toFixed(1)+" MB":Math.max(1,Math.round(b/1e3))+" kB";
/** @param {unknown} [body] @param {BodyInit} [raw] */ async function post(/** @type {string} */ u,body,raw){const r=await fetch(u,{method:"POST",body:raw??JSON.stringify(body||{})});const j=await r.json();if(j.error)throw I18N.err(j);return j}
// concentration units: key -> [label, factor to mg/L]; the names carry the unit (std_0.5ppm), the table shows the one chosen in its header
const CU=/** @type {Record<string, [string, number]>} */ ({mgl:["mg/L",1],ugl:["µg/L",1e-3],ngl:["ng/L",1e-6],gl:["g/L",1e3]}),CUN={ppm:"mgl",ppb:"ugl",ppt:"ngl",mgl:"mgl",ugl:"ugl",ngml:"ugl",ngl:"ngl",gl:"gl"};
const CUF=/** @type {Record<string, number>} */ ({ppm:1,ppb:1e-3,ppt:1e-6,mgl:1,ugl:1e-3,ngml:1e-3,ngl:1e-6,gl:1e3});
ST.unit="mgl";
const roundc=(/** @type {number | string | null | undefined} */ v)=>v==null?null:+(+v).toPrecision(6);
// concentration exists only for standards (in an MRM experiment they feed the calibration line)
const isCal=(/** @type {FileRiga} */ f)=>f.type==="standard";
// value from the name, in the unit of the table
const concFrom=(/** @type {FileRiga} */ f)=>f.conc==null||f.kind!=="mrm"?null:roundc(f.cunit&&CUF[f.cunit]?f.conc*CUF[f.cunit]/CU[ST.unit][1]:f.conc);
function mergeFiles(/** @type {FileRiga[]} */ list){const old=Object.fromEntries(ST.files.map((/** @type {FileRiga} */ f)=>[f.name,f]));
  ST.files=list.map(f=>{const o=old[f.name],type=o?o.type:f.type,conc=o?o.conc:concFrom(f),time=type==="blank"?null:o?o.time:f.time;return {...TIPOF.apply(f,f.name),time,type,conc,use:o?o.use:true,
    // gC: concentration read from the name (yellow in the table until the student edits or confirms them)
    gC:o?o.gC:conc!=null,role:o&&o.role,prop:o&&o.prop,ed:o&&o.ed}});
  if(list.some(f=>!old[f.name]))hrMixPreselect();
  // roles and times proposed from the names by the engine (campioni/2)
  sortFiles();renderFiles();RUOLI.propose(()=>ST.files).then((/** @type {unknown} */ c)=>{if(c){sortFiles();renderFiles()}})}
// low- and high-resolution files cannot be opened together: a note offers the choice while both worlds are in the list, and the loading is paused while both are ticked; the most numerous world is preselected when files arrive
const hrMixCount=()=>{const l=ST.files.filter((/** @type {FileRiga} */ f)=>f.kind in GORD);return {lr:l.filter((/** @type {FileRiga} */ f)=>!f.hr).length,hr:l.filter((/** @type {FileRiga} */ f)=>f.hr).length,onLr:l.filter((/** @type {FileRiga} */ f)=>f.use&&!f.hr).length,onHr:l.filter((/** @type {FileRiga} */ f)=>f.use&&f.hr).length}};
const hrMixed=()=>{const c=hrMixCount();return c.onLr>0&&c.onHr>0};
function hrMixPick(/** @type {boolean} */ hr){ST.files.forEach((/** @type {FileRiga} */ f)=>{if(f.kind in GORD)f.use=!!f.hr===hr})}
function hrMixPreselect(){if(!hrMixed())return;const c=hrMixCount();hrMixPick(c.onHr>c.onLr)}
// order of the table: experiment (Full Scan, MS2, MRM, the rest), then blank, standards, samples; inside each, the rule of ordine.js (time, else concentration, else natural name)
const GORD=/** @type {Record<string, number>} */ ({full:0,ms2:1,mrm:2,infusion:3}),TYPORD=/** @type {Record<string, number>} */ ({blank:0,standard:1,sample:2});
// groups of the table: low resolution first, then high resolution (f.hr, from the engine); a file with several parts (Full Scan + MS2: DDA/IDA) has a group of its own, never the one of its MS2 part
const loadGrp=(/** @type {FileRiga} */ f)=>{const b=GORD[f.kind];if(b==null)return {o:30,key:"other"};const mx=!!(f.mixed&&f.mixed.length>1),hr=!!f.hr;return {o:(hr?10:0)+(mx?4:b),key:(hr?"hr:":"lr:")+(mx?"mix:"+mixedText(f.mixed):f.kind)}};
function sortFiles(){const c=fileComparator(ST.files,(/** @type {FileRiga} */ f)=>f.conc==null?null:f.conc*CU[ST.unit][1]);ST.files.sort((/** @type {FileRiga} */ a,/** @type {FileRiga} */ b)=>{const x=loadGrp(a),y=loadGrp(b);return x.o-y.o||(x.key<y.key?-1:x.key>y.key?1:0)||TYPORD[a.type]-TYPORD[b.type]||c(a,b)})}
// the formats of instrument methods the program reads TODAY (one line per format; the field, the file picker and the texts all come from here)
const METHOD_FORMATS=[{ext:".dam",maker:"SCIEX"}];
const isMethodFile=(/** @type {string} */ name)=>METHOD_FORMATS.some(x=>name.toLowerCase().endsWith(x.ext));
(()=>{const fm=METHOD_FORMATS.map(x=>x.ext+" "+x.maker).join(", ");$("#pickdam").accept=METHOD_FORMATS.map(x=>x.ext).join(",");
  $("#dropdam").setAttribute("aria-label",I18N.t("load.method.drop")+" ("+fm+")");})();
// each method is a chip «Method: name» with ×; the method goes with the data file of the same base name, else with the selected file (api.method)
function renderMethods(){const el=$("#mlist");
  el.innerHTML=ST.methods.map((/** @type {{ name: string, info?: { key?: string, params?: unknown }, size: number }} */ m,/** @type {number} */ i)=>`<span class="chip"><span>${I18N.t("load.method.chip",{name:esc(m.name)})}</span> <span class="muted sm">${esc(m.info&&m.info.key?I18N.t(m.info.key,m.info.params):"")} · ${sz(m.size)}</span> <button type="button" data-mrm="${i}" aria-label="${esc(I18N.t("load.method.remove",{name:m.name}))}" title="${I18N.t("load.table.remove")}">&times;</button></span>`).join("");
  el.querySelectorAll("[data-mrm]").forEach((/** @type {ElementoDati} */ b)=>b.onclick=async()=>{try{const j=await post("api/remove",{name:ST.methods[+b.dataset.mrm].name});mergeFiles(j.files);ST.methods=j.methods||[];renderMethods()}catch(e){fail(e)}})}
const EXPN=/** @type {Record<string, string>} */ ({full:"Full Scan",ms2:"MS2 (Product Ion)",mrm:"MRM",infusion:I18N.t("inf.name")}),expn=(/** @type {string} */ k)=>k==="empty"?I18N.t("load.exp.empty"):EXPN[k]||k;
// Full Scan + MS2 = DDA; polarities and precursor counts are not shown
const mixedText=(/** @type {{ name: string }[]} */ m)=>{const ns=/** @type {string[]} */ ([]);for(const p of m){const n=p.name==="MS2"&&ns.includes("Full Scan")?"DDA":p.name;if(!ns.includes(n))ns.push(n)}return ns.join(" + ")};
// the experiment type is a plain menu of the types with the current one selected (the detected one unless the user picked another, remembered per file); the detection reason is the tooltip
const expSel=(/** @type {FileRiga} */ f,/** @type {string} */ attr,/** @type {string | null | undefined} */ cur,/** @type {string} */ auto,/** @type {string} */ tip)=>`<select ${attr} data-auto="${esc(auto)}" class="sm" aria-label="${I18N.t("load.table.experiment")}" title="${esc(tip)}">${TIPOF.KINDS.map((/** @type {string} */ k)=>`<option value="${k}" ${k===(cur||auto)?"selected":""}>${expn(k)}</option>`).join("")}</select>`;
const expCell=(/** @type {FileRiga} */ f)=>f.kind&&f.mixed&&f.mixed.length?`<b>${esc(mixedText(f.mixed))}</b>`:f.kind in GORD?expSel(f,`data-exp="${esc(f.name)}"`,TIPOF.manual(f.name),f.kindAuto||f.kind,TIPOF.why(f))+(f.polarity==="mixed"?` <span class="muted sm">${I18N.t("load.exp.polMixed")}</span>`:""):f.kind?`<b>${expn(f.kind)}</b>`:`<span class="muted">${I18N.t("load.exp.unread")}</span>`;
// the types of the files change in place (table, viewer, XIC button) and the table is sorted again
function expSet(/** @type {string[]} */ names,/** @type {string | null | undefined} */ kind){names.forEach(n=>TIPOF.set(n,kind||null));ST.files.forEach((/** @type {FileRiga} */ f)=>{if(names.includes(f.name))Object.assign(f,TIPOF.apply(f,f.name))});TIPOF.viewer(names);CONVROWS.thaw();sortFiles();renderFiles()}
function renderFiles(){const F=ST.files,el=$("#flist");
  const hasCal=true,tu=tunitGet(),tuSel=`<select id="tunit" title="${I18N.t("load.table.tunit.title")}" aria-label="${I18N.t("load.table.tunit.title")}">${Object.keys(TIME_UNITS).map(k=>`<option value="${k}" ${k===tu?"selected":""}>${tunitLabel(k)}</option>`).join("")}</select>`,unitSel=`<select id="cunit" title="${I18N.t("load.table.unit.title")}">${Object.entries(CU).map(([k,v])=>`<option value="${k}" ${k===ST.unit?"selected":""}>${v[0]}</option>`).join("")}</select>`;
  // GUESS: concentration cells
  const ncol=7+(hasCal?1:0),GUESS=` title="${I18N.t("load.guess.title")}"`;
  let rows="";
  // the rows keep the order of arrival (CONVROWS.rank) from the first moment on: no group lines, no reorder when the loading ends; the group of a file is the experiment cell and the HR tag
  F.map((/** @type {FileRiga} */ f,/** @type {number} */ i)=>({f,i})).sort((/** @type {{ f: FileRiga }} */ a,/** @type {{ f: FileRiga }} */ b)=>CONVROWS.rank(a.f.name)-CONVROWS.rank(b.f.name)).forEach((/** @type {{ f: FileRiga, i: number }} */ {f,i})=>{if(CONVROWS.holds(f.name))return;
    rows+=`<tr data-grp="${loadGrp(f).key}"><td><input type="checkbox" data-i="${i}" data-k="use" ${f.use?"checked":""}></td><td>${esc(f.name)}${f.hr?` <span class="muted sm">${I18N.t("load.row.hr")}</span>`:""}${CONVQ.label(f.name)?` <span class="muted sm" data-cvq="${esc(f.name)}">${esc(CONVQ.label(f.name))}</span>`:""} ${typeof polSign==="function"?polSign(f,F):""}</td><td>${expCell(f)}</td>`+
   RUOLI.roleCell(f,i)+RUOLI.timeCell(f,i,tu)+
   (hasCal?(isCal(f)?`<td class="num"><input type="text" inputmode="decimal" autocomplete="off" style="width:90px" data-i="${i}" data-k="conc" value="${f.conc??""}" ${f.gC&&f.conc!=null?`class="guess"${GUESS}`:`title="${I18N.t("load.table.concStd.title")}"`}></td>`:f.type==="blank"?`<td class="num"><input type="text" disabled aria-label="${I18N.t("load.table.conc")}" style="width:90px" data-i="${i}" data-k="conc" value=""></td>`:"<td></td>"):"")+
   `<td class="num muted">${sz(f.size)}</td><td style="white-space:nowrap">${CONV.has(f.name)?`<button type="button" class="sec sm" data-dl="${esc(f.name)}" title="${I18N.t("conv.dl.title")}">${I18N.t("conv.dl")}</button> `:""}<button data-rm="${i}" title="${I18N.t("load.table.remove")}">&times;</button></td></tr>`});
  rows+=CONVROWS.html();
  // the columns, the unit menus and the buttons over the table are there from the first row (CONVROWS.table, tools): the phases of the loading move nothing
  el.innerHTML=(F.length||CONVROWS.running().length?CONVROWS.tools()+CONVROWS.table(`<tr><th></th><th scope="col">${I18N.t("load.table.file")}</th><th scope="col">${I18N.t("load.table.experiment")}</th><th scope="col">${I18N.t("load.table.type")}</th><th scope="col" class="num">${I18N.t("load.table.time.col")} ${tuSel}</th><th scope="col" class="num">${I18N.t("load.table.conc")} ${unitSel}</th><th scope="col" class="num">${I18N.t("load.table.size")}</th><th></th></tr>`,rows):"")+CONVROWS.summary();
  CONVROWS.wire(el);
  el.querySelectorAll("[data-exp]").forEach((/** @type {CampoDati} */ x)=>x.onchange=()=>expSet([x.dataset.exp],x.value===x.dataset.auto?null:x.value));
  el.querySelectorAll("[data-k]").forEach((/** @type {CampoDati} */ x)=>{
    x.onchange=()=>{const f=F[+x.dataset.i],k=x.dataset.k,keep=document.activeElement===x?{name:f.name,k}:null;let v=x.value;
      if(k==="time"||k==="conc"){const dk=k==="time"&&RUOLI.roleOf(f)==="dark";v=parseDecimal(x.value,dk);if(Number.isNaN(v)){x.setAttribute("aria-invalid","true");x.title=I18N.t("load.table.num.bad");x.classList.add("bad");return}x.removeAttribute("aria-invalid");if(k==="time")v=dk?darkTime(timeStored(v,tunitGet())):timeStored(v,tunitGet())}
      if(k==="type"){f.ed=1;RUOLI.setRole(f,x.value)}else f[k]=k==="use"?x.checked:v;
      if(k==="conc")f.gC=false;
      if(k==="type"){if(isCal(f)&&f.conc==null){f.conc=concFrom(f);f.gC=f.conc!=null}if(!isCal(f)){f.conc=null;f.gC=false}}
      // the table is sorted again when a value is committed (change/blur/Enter), never while typing. A change caused by leaving the field (click on another field, Tab) is applied
      // once the focus has moved, and the focus goes back to the field that received it: the rebuilt table must not swallow the click
      const go=()=>{const a=/** @type {HTMLElement} */(document.activeElement),ok=a&&el.contains(a),nk=/** @type {{ name: string, k?: string, rm?: number } | null} */ (ok&&a.dataset.k?{name:F[+(a.dataset.i||0)].name,k:a.dataset.k}:ok&&a.dataset.rm?{name:F[+a.dataset.rm].name,rm:1}:keep);CONVROWS.thaw();sortFiles();renderFiles();
        if(nk){const i=F.findIndex((/** @type {FileRiga} */ q)=>q.name===nk.name),b=el.querySelector(nk.rm?`[data-rm="${i}"]`:`[data-i="${i}"][data-k="${nk.k}"]`);if(b){b.focus();if(b.type==="text")b.setSelectionRange(b.value.length,b.value.length)}}};
      if(keep)go();else setTimeout(go,0)};
    if(x.dataset.k==="conc")x.addEventListener("keydown",(/** @type {KeyboardEvent} */ e)=>{if(e.key==="Enter"){F[+x.dataset.i].gC=false;x.classList.remove("guess");x.title="";modeNote()}})});
  el.querySelectorAll("[data-dl]").forEach((/** @type {ElementoDati} */ b)=>b.onclick=()=>CONV.download(b.dataset.dl));const npc=$("#npress");if(npc)npc.onchange=()=>{CONV.numpress=npc.checked};const dla=$("#dlall");if(dla)dla.onclick=()=>CONV.downloadAll();const sva=$("#saveall");if(sva)sva.onclick=()=>CONVQ.saveAll();
  const tuEl=$("#tunit");if(tuEl)tuEl.onchange=()=>{tunitSet(tuEl.value);renderFiles()};
  const cu=$("#cunit");if(cu)cu.onchange=()=>{const k=cu.value,r=CU[ST.unit][1]/CU[k][1];F.forEach((/** @type {FileRiga} */ f)=>{if(f.conc!=null)f.conc=roundc(f.conc*r)});ST.unit=k;renderFiles()};
  el.querySelectorAll("[data-rm]").forEach((/** @type {ElementoDati} */ b)=>b.onclick=async()=>{try{const f=F[+b.dataset.rm];const j=await post("api/remove",{name:f.name});mergeFiles(j.files)}catch(e){fail(e)}});checkGo();modeNote()}
function hrMixNote(){const n=$("#hrmix");if(!n)return;const c=hrMixCount();n.hidden=!(c.lr>0&&c.hr>0);if(n.hidden)return;
  $("#hrmix-t").textContent=I18N.t("load.hrmix.note");$("#hrmix-lr").textContent=I18N.t("load.hrmix.lr",{n:c.lr});$("#hrmix-hr").textContent=I18N.t("load.hrmix.hr",{n:c.hr});
  $("#hrmix-lr").setAttribute("aria-pressed",String(c.onLr>0&&c.onHr===0));$("#hrmix-hr").setAttribute("aria-pressed",String(c.onHr>0&&c.onLr===0))}
function modeNote(){hrMixNote();const h=$("#hrnote");if(h)h.hidden=!ST.files.some((/** @type {FileRiga} */ f)=>f.use&&f.hr_profile);const m=$("#modenote");if(!m)return;const u=ST.files.filter((/** @type {FileRiga} */ f)=>f.use&&["full","ms2"].includes(f.kind)),has=(/** @type {string} */ x)=>u.some((/** @type {FileRiga} */ f)=>f.spectrum_mode===x||f.spectrum_mode==="mixed");m.hidden=!(has("profile")&&has("centroid"))}
// the button is live only when at least one chosen file can be opened (an mzML with no data cannot)
const usable=(/** @type {FileRiga} */ f)=>f.use&&["full","ms2","mrm","infusion"].includes(f.kind);
// ...and not while files are still being read and indexed: the button would open only the ones already ready (UPBUSY = loadings under way)
// names of the files that could not be read: mzFinder warns that they are missing from its series
let UPBUSY=0;const LOADFAIL=window.QQQ_LOADFAIL=new Set();
// hook for a progress bar on the loading screen: loadingStato("phase", 40) shows the phase and 40 % in place of the phrase, loadingStato(null) gives the phrases back
/** @param {number | null} [pct] */ function loadingStato(/** @type {string | null} */ text,pct){ldMsg=text?{text,pct:pct==null?null:pct}:null;ldRefresh()}
// the button waits for the files still being read: its click shows the overlay with the phase and the percentage of the whole batch (event mzlab-index), then opens them all
// the numbers come from PROGRESSO (progresso.js): one model for the card, the overlay and Tabui
const IDLE=/** @type {((v?: unknown) => void)[]} */ ([]);let WAITING=false;
let WAIT0=0;      // when the click started waiting: the overlay shows the time, so it is never still
function waitStato(){if(!WAITING)return;const s=PROGRESSO.state(),on=PROGRESSO.busy(),lab=I18N.t(on?PROG_PHASE_KEY[s.phase]||"load.ph.index":"load.ph.prep"),k=on&&s.n>1?" · "+I18N.t("load.prog.count",{done:s.done,n:s.n}):"";
  loadingStato(`${lab}${k} · ${convElapsed(Date.now()-WAIT0)}`,on?s.pc:null)}
PROGRESSO.subscribe(waitStato);
const idle=()=>UPBUSY>0?new Promise(ok=>IDLE.push(ok)):Promise.resolve();
function checkGo(){$("#opbtn").disabled=(UPBUSY===0&&!ST.files.some(usable))||hrMixed();$("#gomsg").textContent=""}
// sends a file and returns the JSON answer (progress callback: loaded, total); in the browser version there is no network: it goes to the worker
const sendFile=(/** @type {string} */ url,/** @type {Blob} */ f,/** @type {(loaded: number, total: number) => void} */ onp)=>window.QQQ_BROWSER?fetch(url,{method:"POST",body:f}).then(r=>{onp(1,1);return r.json()}):new Promise((ok,ko)=>{const x=new XMLHttpRequest();x.open("POST",url);
  x.upload.onprogress=e=>{if(e.lengthComputable)onp(e.loaded,e.total)};x.onload=()=>{let j={};try{j=JSON.parse(x.responseText)}catch(_){}ok(j)};x.onerror=()=>ko(new Error(I18N.t("err.upload.network")));x.send(f)});
const DEMO_SETS=/** @type {Record<string, string[]>} */ ({full:[0,5,10,15,30].map(t=>`FullScan_t${t}.mzML`),ms2:[15,60].map(t=>`MS2_t${t}.mzML`),mrm:["0.6","2.4","7.2","18"].map(c=>`MRM_std_${c}ppm.mzML`),hrms:[0,5,10,30].map(t=>`HRMS_t${String(t).padStart(3,"0")}.mzML`)});
(()=>{const d=$("#demo");if(!d)return;
 const ch=$("#demochoice"),b=$("#demobtn");
 // the button only opens the choice; a set (Full Scan ~28 MB, MS2 ~10 MB, MRM ~2 MB, HRMS ~28 MB) is downloaded only when it is chosen: its rows are in the list at once (upload with `demo`)
 const setOpen=(/** @type {boolean} */ on)=>{ch.hidden=!on;ch.style.display=on?"flex":"none";b.setAttribute("aria-expanded",on?"true":"false")};
 b.onclick=()=>setOpen(ch.hidden);
 ch.querySelectorAll("[data-demo]").forEach((/** @type {ElementoDati} */ o)=>o.onclick=async()=>{setOpen(false);b.disabled=true;try{await upload([],"data",DEMO_SETS[o.dataset.demo])}finally{b.disabled=false}})})();
// files of the instruments are not read by the program (no reader of proprietary formats): a message with the recipe instead of a generic error
// full screen of the whole app (Fullscreen API; the button stays hidden where the browser has none, e.g. iPhone)
(()=>{const b=$("#fs-btn"),de=document.documentElement;if(!b)return;if(!de.requestFullscreen){b.hidden=true;return}
  const sync=()=>{const on=!!document.fullscreenElement,k=on?"header.fullscreen.exit":"header.fullscreen";b.setAttribute("aria-pressed",on);b.querySelector(".fs-in").hidden=on;b.querySelector(".fs-out").hidden=!on;b.title=I18N.t(k);b.setAttribute("aria-label",b.title);b.dataset.i18nTitle=b.dataset.i18nAriaLabel=k};
  b.onclick=()=>{(document.fullscreenElement?document.exitFullscreen():de.requestFullscreen({navigationUI:"hide"})).catch(()=>{})};document.addEventListener("fullscreenchange",sync)})();
(()=>{const p=window.MZLAB_RUST_READY;if(p)p.then((/** @type {{ nativo?: boolean } | null} */ b)=>{const d=$("#drop");if(b&&d&&b.nativo){CONVERT_RAW=true}else $("#convhelp").hidden=false}); else $("#convhelp").hidden=false})()   // the .raw note only where the Rust engine can read it
// a Thermo .raw is read as it is by the Rust engine (rust-nativo.js): each file is a sample `x.mzML` and no mzML is written; without the engine the recipe is shown
async function rawToMzml(/** @type {FileCarica[]} */ files,/** @type {unknown} */ box){const rb=window.MZLAB_RUST_READY?await window.MZLAB_RUST_READY:null;if(!rb||rb.disabled||!rb.nativo||!files.some(f=>/\.raw$/i.test(f.name)))return files;
  return NATIVO.raws(files,(ST.files||[]).map((/** @type {FileRiga} */ x)=>x.name))}
// stem (lower case) -> {stem,wiff,scan}: a file of a SCIEX pair that came without the other, kept for a later loading
const WIFF_WAIT=new Map();
// SCIEX .wiff + .wiff.scan, paired by name (also when they come in different loadings): the Rust engine reads them as they are and each sample (injection) is a file of the list. Gives the files to go on with and the notices [message, name]
async function wiffToMzml(/** @type {FileCarica[]} */ files,/** @type {unknown} */ box,/** @type {AbortSignal} */ signal){const rb=window.MZLAB_RUST_READY?await window.MZLAB_RUST_READY:null,res=/** @type {any} */ ({files,errs:[]}),isW=(/** @type {FileCarica} */ f)=>/\.wiff(\.scan)?$/i.test(f.name);
  // the engine exists but cannot convert now (it stopped twice in a minute, or the page and its scripts are of two versions): say so, not «cannot be opened in the browser»
  if((rb?(rb.disabled||!rb.nativeSamples):window.MZLAB_RUST_ERR&&!window.QQQ_PHONE)&&files.some(isW)){
    res.files=files.filter(f=>!isW(f));files.filter(isW).forEach(f=>res.errs.push([I18N.t(rb?"err.file.wiffEngine":"err.file.wiffLoad"),f.name]));return res}
  if(!rb||rb.disabled||!rb.nativeSamples||!files.some(isW))return res;
  res.files=files.filter(f=>!isW(f));const T=/** @type {(k: string, p?: unknown) => string} */ ((k,p)=>I18N.t(k,p)),dropped=new Set();
  for(const f of files.filter(isW)){const sc=/\.scan$/i.test(f.name),stem=f.name.replace(/\.wiff(\.scan)?$/i,""),k=CartCarica.pairKey(f,stem),e=WIFF_WAIT.get(k)||{stem};e[sc?"scan":"wiff"]=f;WIFF_WAIT.set(k,e);dropped.add(k)}
  const taken=[...res.files.map((/** @type {FileRiga} */ x)=>x.name),...(ST.files||[]).map((/** @type {FileRiga} */ x)=>x.name)],pairs=[];
  // a pair is in the list as soon as its two files are together, before anything is read
  for(const [k,e] of [...WIFF_WAIT]){
    // a .wiff alone: after the loading it may turn out to be the method of an mzML (loneWiffs)
    if(!(e.wiff&&e.scan)){if(dropped.has(k)){if(e.wiff)(res.lone=res.lone||[]).push({e,rb});else res.errs.push([T("load.wiffWait",{name:e.scan.name}),e.scan.name])}continue}
    WIFF_WAIT.delete(k);e.row=e.stem+".wiff";pairs.push(e);CONVROWS.add(e.row,"open",memDesc([e.wiff,e.scan]))}
  // each sample (injection) is a file of the list: the .wiff and the .wiff.scan are read as they are
  for(const e of pairs){
    if(signal&&signal.aborted){CONVROWS.finish(e.row,"cancel");continue}
    // the module looks at every pair first: the engine reads the 3200 QTRAP only, and a file of another instrument or scan type is for the module («variante»)
    try{const mem=[{name:e.wiff.name,blob:e.wiff},{name:e.scan.name,blob:e.scan}],M0=window.LETTORI_MOD,pr=M0&&window.LETTORI_FAM.sciex.sonda?await M0.probe("sciex",mem).catch(()=>null):null;
      if(pr&&pr.motivi.includes("variante")){const cs=pr.campioni.length?pr.campioni:[{id:1}];cs.forEach((/** @type {{ id: number, name?: string, size?: number }} */ x,/** @type {number} */ i)=>{const nm=wiffName(e.stem,x.id,cs.length>1,taken,x.name);taken.push(nm);res.files.push(M0.stand(nm,mem,i,"sciex",x.size))});CONVROWS.drop(e.row);continue}
      const hit=await NATIVO.cached(rb,e.stem,mem,ST.files||[]);if(hit){for(const x of hit){CONVROWS.add(x.name,"open");CONVROWS.finish(x.name,"done",{scans:x.n_spectra||x.scans||0,kind:(x.mixed||[]).length>1?"mixed":x.kind,missing:(/** @type {any} */ (window).MZLAB_LR?.missing?.get(x.name))||0,cached:true})}CONVROWS.drop(e.row);continue}      // the same pair again: its samples are in the list with their index, nothing is copied or read again
      const s=(await rb.nativeSamples(mem)).samples,key=NATIVO.key(e.stem);
      // the module's answer for the pair holds for its samples (moduloWants)
      s.forEach((/** @type {{ id: number, name?: string, size?: number }} */ x,/** @type {number} */ i)=>{const nm=wiffName(e.stem,x.id,s.length>1,taken,x.name),f=NATIVO.sample(nm,key,mem,i,x.id,false,x.size);taken.push(nm);if(pr)f.mzlabNat.probed=pr;res.files.push(f)})}
    catch(x_){const x=/** @type {Lanciato} */(x_);const M=window.LETTORI_MOD,fam=M&&x.json?M.retriable(x.json.error||x.json.error_key):"";if(fam)res.files.push(M.stand(wiffName(e.stem,1,false,taken),[{name:e.wiff.name,blob:e.wiff},{name:e.scan.name,blob:e.scan}],0,fam));
      // the pair cannot be read: the error stays on its row, with «Details» and the trace
      else{const er=x.json?I18N.err(x.json):x;CONVROWS.finish(e.row,"fail",{msg:er.message,diag:DIAG.da(er,{name:e.scan.name,size:e.wiff.size+e.scan.size})});continue}}
    CONVROWS.drop(e.row)}
  return res}
// Rule R (docs/agenti/lettori-modulo.md): the readers module (lettori-modulo.js) writes the mzML of what the engine does not read, into OPFS, and the mzML enters upload() as any other: a file with an extension of the
// module, a sample the engine refused with an error of variant (File with `mzlabModulo`, made by LETTORI_MOD.stand), an mzML with numpress arrays, a dataset the module's probe says the engine cannot do, and, with ?lettori=modulo, a .D folder (proof of the path).
const LETT_FORCE=/[?&]lettori=modulo\b/.test(location.search);
/** the family whose module takes `f` ("" = none): a sample the engine refused, a file with an extension only a module reads, a dataset the module of its family says the engine cannot do (probe) */
async function moduloWants(/** @type {typeof LETTORI_MOD} */ M,/** @type {FileCarica} */ f){
  if(f.mzlabModulo)return f.mzlabModulo.fam;const n=f.name.toLowerCase();
  if(M.extensions().some((/** @type {string} */ e)=>n.endsWith(e)))return M.famigliaDi([n]);
  // an mzML with numpress arrays: the engine refuses it (err.file.numpress), the module writes it again with zlib
  if(n.endsWith(".mzml")&&!f.mzlabNat){const v=await M.variante(f);if(v)return v}
  const nat=f.mzlabNat;if(!nat)return "";const fam=M.famigliaDi(nat.members.map((/** @type {{ name: string }} */ m)=>m.name));if(!fam)return "";
  if(LETT_FORCE&&nat.members.some((/** @type {{ name: string }} */ m)=>/\.d\//i.test(m.name)))return fam;
  if(!M.loaded)return "";
  // a sample of a .wiff: the pair was probed once (wiffToMzml); a full read of the .wiff.scan per sample made the loading wait minutes
  if(nat.probed)return nat.probed.motivi.length>0?fam:"";
  try{return (await M.probe(fam,nat.members)).motivi.length>0?fam:""}catch(_){return ""}}
async function moduloToMzml(/** @type {FileCarica[]} */ files,/** @type {unknown} */ box,/** @type {CaricaRep} */ rep){
  const M=window.LETTORI_MOD;if(!M||!files.length)return files;
  const jobs=new Map();for(const f of files){const fam=await moduloWants(M,f);if(fam)jobs.set(f,fam)}if(!jobs.size)return files;
  const g=ConvUI.group(box,[...jobs.keys()].map(f=>NATIVO.size(f))),out=/** @type {any} */ ([]),taken=[...(ST.files||[]).map((/** @type {FileRiga} */ x)=>x.name),...files.filter(f=>!jobs.has(f)).map(f=>f.name)];
  // every file of the batch has its row (Queued) from the start, so the count over the table is the count of the headline
  const names=new Map();for(const f of jobs.keys()){const lot=f.mzlabModulo||f.mzlabNat,nm=lot?f.name:M.mzmlName(f.name,taken);taken.push(nm);names.set(f,nm);CONVROWS.add(nm,"wait")}
  let i=0;
  for(const f of files){
    if(!jobs.has(f)){out.push(f);continue}
    i++;const lot=f.mzlabModulo||f.mzlabNat,mem=lot?lot.members:[{name:f.name,blob:f}],nm=names.get(f);
    if(g.signal.aborted)continue;
    CONVROWS.set(nm,"open");const ui=g.start(i,nm),dest=`${Date.now().toString(36)}-${Math.random().toString(36).slice(2,8)}.mzML`;
    try{const blob=await M.toMzml(jobs.get(f),mem,lot?lot.c:0,dest,(/** @type {{ scans: number, scansTotal: number }} */ p)=>{ui.update(p);CONVROWS.set(nm,"run",p.scansTotal?Math.floor(100*p.scans/p.scansTotal):0)},g.signal);
      ui.done();CONV.add(nm,blob);out.push(new File([blob],nm))}
    catch(e_){const e=/** @type {Lanciato} */(e_);ui.done();if(!e.cancelled){const x=e.json?I18N.err(e.json):e,m=x.message;(rep.errs[m]=rep.errs[m]||[]).push(f.name);if(!(rep.diag||(rep.diag={}))[m])rep.diag[m]=DIAG.da(Object.assign(x,{fase:x.fase||"decode"}),{name:f.name,size:NATIVO.size(f)},await DIAG.leggiTesta(f.mzlabNat||f.mzlabModulo?new Blob([]):f))}}
    finally{CONVROWS.drop(nm)}}
  for(const nm of names.values())CONVROWS.drop(nm);
  out.panel=g;return out}
// true where the engine converts .raw itself (desktop, ?motore=rust): the guide then appears only for the formats it cannot convert
let CONVERT_RAW=false;
const UNOPEN_RE=/\.(raw|wiff2?|wiff\.scan|d|lcdx)$/i;
function fileKind(/** @type {string} */ name){const n=name.toLowerCase();if(/\.(mzml|dam)$/.test(n))return null;const M=window.LETTORI_MOD;if(M&&M.extensions().some((/** @type {string} */ e)=>n.endsWith(e)))return null;const m=UNOPEN_RE.exec(n);return m?m[0]:"other"}
function ssGet(/** @type {string} */ k){try{return sessionStorage.getItem(k)}catch(e){return null}}
function ssSet(/** @type {string} */ k,/** @type {string} */ v){try{sessionStorage.setItem(k,v)}catch(e){}}
const SEEN_GROUPS=new Set();
function groupHash(/** @type {{ name: string, size: number }[]} */ files){const s=files.map(f=>f.name+":"+f.size).sort().join("|");let h=5381;for(let i=0;i<s.length;i++)h=(h*33^s.charCodeAt(i))>>>0;return h+"."+files.length}
function groupSeen(/** @type {string} */ h){let a=[];try{a=JSON.parse(ssGet(NOMI.KEY("unopen"))||"[]")}catch(e){}return SEEN_GROUPS.has(h)||a.includes(h)}
function groupMark(/** @type {string} */ h){SEEN_GROUPS.add(h);try{const a=JSON.parse(ssGet(NOMI.KEY("unopen"))||"[]");a.push(h);ssSet(NOMI.KEY("unopen"),JSON.stringify(a.slice(-50)))}catch(e){}}
function namesCut(/** @type {string[]} */ a,/** @type {number} */ max){return a.slice(0,max).map(esc).join(", ")+(a.length>max?"…":"")}
// ONE notice for the whole loading: files that cannot be opened in the browser (count + collapsible list), unknown formats and read errors, one line per kind
function closeUnopen(){const u=$("#unopen");u.hidden=true;u.textContent=""}
function openConvHelp(/** @type {boolean} */ focus){const d=$("#convhelp");d.hidden=false;d.open=true;if(focus){d.scrollIntoView({block:"nearest"});d.querySelector("summary").focus()}}
function reportLoad(/** @type {CaricaRep} */ rep){
  const u=$("#unopen"),L=/** @type {string[]} */ ([]),T=/** @type {(k: string, p?: unknown) => string} */ ((k,p)=>I18N.t(k,p));let conv=/** @type {string[]} */ ([]);
  if(rep.unopen.length){
    const by=/** @type {Record<string, number>} */ ({});rep.unopen.forEach(x=>{by[x.ext]=(by[x.ext]||0)+1;conv.push(x.ext)});
    const types=Object.keys(by).join(", "),n=rep.unopen.length,names=rep.unopen.map(x=>x.name);
    L.push(`<p><b>${T(n===1?"unopen.head.one":"unopen.head",{n,types})}</b></p>`);
    L.push(`<details class="unopen-names"><summary>${T("unopen.names")}</summary><ul>${names.slice(0,5).map(x=>`<li>${esc(x)}</li>`).join("")}${names.length>5?`<li class="muted">${T("unopen.more",{n:names.length-5})}</li>`:""}</ul></details>`)}
  if(rep.other.length)L.push(`<p>${T("unopen.other",{n:rep.other.length,names:namesCut(rep.other,3)})}</p>`);
  const DG=/** @type {unknown[]} */ ([]);
  for(const [msg,names] of Object.entries(rep.errs)){const d=(rep.diag||{})[msg],why=d&&d.causa!=="unknown"?` ${I18N.t(`diag.cause.${d.causa}`)}`:"";if(d)DG.push(d);
    L.push(`<div class="fail"><p>${names.length===1?T("unopen.err.one",{name:esc(names[0]),message:esc(msg)}):T("unopen.err.many",{n:names.length,names:namesCut(names,3),message:esc(msg)})}${esc(why)}</p>${d?DIAG.html(d,esc,T):""}</div>`)}
  if(rep.moved.dam.length)L.push(`<p>${T("load.note.movedToDam",{name:namesCut(rep.moved.dam,3)})}</p>`);
  if(rep.moved.data.length)L.push(`<p>${T("load.note.movedToData",{name:namesCut(rep.moved.data,3)})}</p>`);
  for(const n of rep.notes||[])L.push(`<p>${esc(n)}</p>`);
  if(!L.length)return;
  const needHelp=rep.unopen.length||rep.other.length;
  u.innerHTML=L.join("")+`<div class="row">${needHelp?`<button type="button" class="sec" data-a="help">${T("unopen.how")}</button>`:""}<button type="button" class="sec" data-a="close">${T("common.close")}</button></div>`;
  u.hidden=false;
  u.querySelector('[data-a=close]').onclick=closeUnopen;
  const hb=u.querySelector('[data-a=help]');if(hb)hb.onclick=()=>openConvHelp(true);
  u.querySelectorAll(".diag").forEach((/** @type {HTMLElement} */ el,/** @type {number} */ i)=>DIAG.cabla(el,DG[i],window.MZLAB_VERSION,T));
  if(rep.unopen.length){
    const h=$("#convhint");h.textContent=T("conv.help.hint",{types:[...new Set(conv)].join(", ")});h.hidden=false;
    if(!ssGet(NOMI.KEY("convopen"))){ssSet(NOMI.KEY("convopen"),"1");openConvHelp(false)}   // opens by itself once per session
    if(CONVERT_RAW)$("#convdesk").hidden=false}
  else if(needHelp&&CONVERT_RAW)$("#convhelp").hidden=false}
document.addEventListener("keydown",e=>{if(e.key==="Escape"&&!$("#unopen").hidden&&$("#unopen").contains(document.activeElement)){e.preventDefault();closeUnopen()}});
// a .wiff without its .scan is the method of the mzML files made from it (same SHA-1): else it waits for its .scan
async function loneWiffs(/** @type {{ lone?: { e: { stem: string, wiff: File }, rb: any }[] }} */ wr,/** @type {CaricaRep} */ rep){
  for(const {e,rb} of wr.lone||[]){try{const r=await rb.metodoWiff(e.wiff);if(r.matched.length){WIFF_WAIT.delete(e.stem.toLowerCase());rep.notes.push(I18N.t("load.note.wiffMethod",{name:e.wiff.name,n:r.matched.length,files:namesCut(r.matched.map((/** @type {{ name: string }} */ x)=>x.name),3)}));continue}}catch(_){}
    const m=I18N.t("err.file.wiffNoScan",{name:e.wiff.name});(rep.errs[m]=rep.errs[m]||[]).push(e.wiff.name)}}
const INFLIGHT=new Set(),fkey=(/** @type {FileRiga} */ f)=>f.name+"|"+f.size;
// the same file (name and size) already in the list, or being read now, is not loaded again: a short notice and its row lights up for a moment
function dropDups(/** @type {FileCarica[]} */ files){const dups=files.filter(f=>INFLIGHT.has(fkey(f))||ST.files.some((/** @type {FileRiga} */ g)=>g.name===f.name&&g.size===f.size));if(!dups.length)return files;
  const msg=dups.length===1?I18N.t("load.dup.one",{name:dups[0].name}):I18N.t("load.dup.many",{n:dups.length});CartCarica.note(msg);setTimeout(()=>{const n=$("#cartinfo");if(n&&n.textContent===msg)CartCarica.note("")},5000);
  dups.forEach(f=>{const tr=[...document.querySelectorAll("#flist tr")].find(r=>r.children[1]&&r.children[1].textContent.trim().startsWith(f.name));if(tr){tr.classList.add("dupflash");tr.scrollIntoView({block:"nearest"});setTimeout(()=>tr.classList.remove("dupflash"),1800)}});
  return files.filter(f=>!dups.includes(f))}
// the AbortControllers of the loadings under way: «Cancel» (rows of the list, overlay of «Load data») stops them all
const LOADS=new Set();
// the members of a file without their names, for the diagnosis: ".wiff 1.2 MB + .wiff.scan 57.0 MB"
const memDesc=(/** @type {{ name: string, size: number }[]} */ fs)=>fs.map(b=>`${(String(b.name).match(/\.[^./\\]+(\.scan)?$/i)||["?"])[0]} ${(b.size/1048576).toFixed(1)} MB`).join(" + ");
// «Cancel»: the files not started are not read, the engine stops the copies and the indexes running (lr_cancel), the module its conversions; the files already loaded stay
async function cancelLoads(){for(const c of LOADS)c.abort();if(CONVQ.on())CONVQ.stop();const rb=window.MZLAB_RUST,names=CONVROWS.running().map((/** @type {{ name: string }} */ r)=>r.name);if(rb&&rb.rpc&&names.length)try{await rb.rpc({op:"lr_cancel",names})}catch(_){}}
// downloads the example files in order: the bytes are the first 40 % of the bar of each row (Content-Length); a cancel stops the rows, an error drops them and says why
async function fetchDemo(/** @type {string[]} */ names,/** @type {AbortSignal} */ signal){const out=[];
  try{for(const n of names){CONVROWS.set(n,"get",0);const r=await fetch("static/esempi/"+n,{signal});if(!r.ok)throw new Error(n);const len=+(r.headers.get("content-length")||0),parts=[];let got=0;
      if(r.body&&r.body.getReader){const rd=r.body.getReader();for(;;){const {done,value}=await rd.read();if(done)break;parts.push(value);got+=value.length;CONVROWS.set(n,"get",len?Math.floor(40*Math.min(1,got/len)):0,undefined,{done:got,total:len})}}else parts.push(await r.blob());
      CONVROWS.set(n,"wait",40);CONVROWS.base(n,40);out.push(new File(parts,n))}}
  catch(e_){const e=/** @type {Lanciato} */(e_);if(signal.aborted)names.forEach(n=>CONVROWS.finish(n,"cancel"));else{names.forEach(n=>CONVROWS.drop(n));fail(I18N.t("load.demo.error",{message:e.message}))}return []}
  return out}
// `demo`: names of example files still to download (static/esempi): their rows are in the list at once, in order, and the download is the first part of each row
/** @param {FileCarica[]} files @param {string} into @param {string[]} [demo] */
async function upload(files,into,demo){if(typeof AVVIO!=="undefined"&&!(await AVVIO.pronto()))return;const box=$("#up");fail();
  if(demo){demo=demo.filter(n=>!ST.files.some((/** @type {FileRiga} */ g)=>g.name===n)&&!INFLIGHT.has(n));if(!demo.length)return;files=[];demo.forEach(n=>INFLIGHT.add(n))}else{files=dropDups(files);if(!files.length)return;files.forEach(f=>INFLIGHT.add(fkey(f)))}
  if(!UPBUSY)CONVROWS.clearDone();UPBUSY++;checkGo();let mine=files;const ctl=new AbortController();LOADS.add(ctl);CONVROWS.onCancel=cancelLoads;if(demo)demo.forEach(n=>{CONVROWS.add(n,"wait");CONVROWS.base(n,0)});
  // for the diagnosis of the loading
  try{navigator.storage.estimate().then(e=>{window.MZLAB_STIMA=`${Math.round((e.usage||0)/1e6)} of ${Math.round((e.quota||0)/1e6)} MB`}).catch(()=>{})}catch(_){}
  try{if(demo){files=await fetchDemo(demo,ctl.signal);files.forEach(f=>INFLIGHT.add(fkey(f)));mine=files}await upload0(files,into,box,ctl.signal)}finally{LOADS.delete(ctl);mine.forEach(f=>INFLIGHT.delete(fkey(f)));(demo||[]).forEach(n=>INFLIGHT.delete(n));UPBUSY--;checkGo();if(!UPBUSY){CONVROWS.onCancel=null;CONVROWS.paint();IDLE.splice(0).forEach(ok=>ok())}}}
async function upload0(/** @type {FileCarica[]} */ files,/** @type {string | undefined} */ into,/** @type {unknown} */ box,/** @type {AbortSignal} */ signal){
  const rep=/** @type {CaricaRep} */ ({unopen:[],other:[],errs:{},moved:{dam:[],data:[]},notes:[]}),rejected=/** @type {FileCarica[]} */ ([]);
  const one=async (/** @type {FileCarica} */ f)=>{const isDam=isMethodFile(f.name);
    if(signal.aborted){CONVROWS.finish(f.name,"cancel");PROGRESSO.end(f.name);return}
    if(into&&isDam!==(into==="dam"))rep.moved[isDam?"dam":"data"].push(f.name);
    // a converted file: its index is the last phase of the bar of the conversion
    const pn=panel&&CONV.has(f.name)?panel:null,QQQ_NET=!window.QQQ_BROWSER,mono=()=>Math.floor(100*PROGRESSO.file(f.name));
    // the bar of the row: load 0-40, read 40-85, index 85-100 (rowPc, progresso.js) whatever the reader; the same fraction of the model always gives the same percent
    const rp=()=>Math.min(99,Math.floor(rowPc(!!f.mzlabNat,PROGRESSO.file(f.name)))),rowSt=(/** @type {string} */ phase,/** @type {number} */ pc)=>phase==="load"?"load":f.mzlabNat?NATIVO.rowState(phase):pc>=85?"idx":"nscan";let tot=0;
    CONVROWS.set(f.name,f.mzlabNat?"index":"load",0);if(pn)pn.index(f.name,0);
    /** @param {{ done: number, total: number }} [det] */ const put=(/** @type {number} */ frac,/** @type {string} */ phase,/** @type {{ done: number, total: number } | null | undefined} */ sub,det)=>{PROGRESSO.set(f.name,QQQ_NET?PROG_NET.send+(1-PROG_NET.send)*frac:frac,phase);const pc=rp();CONVROWS.set(f.name,rowSt(phase,pc),pc,sub,det);if(pn)pn.index(f.name,Math.min(99,mono()))};
    // the worker tells the phase, the fraction, the scans of the sample done (done/total) or that the file waits for a free builder (wait); the last message goes to the diagnosis
    const onIdx=(/** @type {Event} */ e)=>{const d=/** @type {CustomEvent} */ (e).detail;if(d.name!==f.name)return;CONVROWS.note(f.name,`${d.phase||"?"} ${(+d.frac||0).toFixed(3)}${d.total?` scans ${d.done}/${d.total}`:""}${d.wait?" queued":""}`);
      if(d.wait){CONVROWS.set(f.name,"wait",rp());return}
      if(d.phase==="load"){put(d.frac,"load",null,{done:d.done,total:d.total});return}
      if(d.total)tot=d.total;      // the scans of an mzML are known from its first event; the builder then tells the position in the file, so the scans read are that share of them
      put(d.frac,d.phase,d.total?{done:d.done,total:d.total}:!f.mzlabNat&&tot?{done:Math.round(tot*Math.max(0,Math.min(1,(d.frac-0.4)/0.585))),total:tot}:null)};
    addEventListener("mzlab-index",onIdx);
    try{const j=await sendFile("api/upload?name="+encodeURIComponent(f.name),f,(/** @type {number} */ a,/** @type {number} */ b)=>{if(QQQ_NET)PROGRESSO.set(f.name,PROG_NET.send*a/b,"send");const pc=Math.min(99,mono());CONVROWS.set(f.name,"index",pc);if(pn)pn.index(f.name,pc)});
      if(j.error)throw I18N.err(j);const x=(j.files||[]).find((/** @type {FileRiga} */ y)=>y.name===f.name);if(f.mzlabNat||x){CONVROWS.finish(f.name,"done",{scans:x?x.n_spectra||x.scans||0:0,kind:x?(x.mixed||[]).length>1?"mixed":x.kind:"",missing:(/** @type {any} */ (window).MZLAB_LR?.missing?.get(f.name))||0})}else CONVROWS.drop(f.name);if(pn)pn.index(f.name,100);mergeFiles(j.files);ST.methods=j.methods||ST.methods;renderMethods();
      LOADFAIL.delete(f.name)}
    catch(e_){const e=/** @type {Lanciato} */(e_);LOADFAIL.add(f.name);if(e.key==="err.load.cancelled"){CONVROWS.finish(f.name,"cancel");return}
      const d=DIAG.da(e,{name:f.name,size:NATIVO.size(f)},await DIAG.leggiTesta(f.mzlabNat||f.mzlabModulo?new Blob([]):f));
      // a sample of a maker's file: the error stays on its row, with «Details» and the trace of the loading
      if(CONVROWS.find(f.name)&&f.mzlabNat){CONVROWS.finish(f.name,"fail",{msg:e.message,diag:d});return}
      CONVROWS.drop(f.name);(rep.errs[e.message]=rep.errs[e.message]||[]).push(f.name);if(!(rep.diag||(rep.diag={}))[e.message])rep.diag[e.message]=d}finally{removeEventListener("mzlab-index",onIdx);PROGRESSO.end(f.name)}};
  // a saved fragmentation graph (g.json) opens by itself, without raw data: infusione-grafo-apri.js
  if(files.some(f=>/\.json$/i.test(f.name))){await carica("m:infusione-grafo-apri.js");files=await INFGAPRI.take(files,rep)}
  let panel=/** @type {{ index(name: string, pc: number): void, end(): void } | null} */ (null);
  const wr=await wiffToMzml(files,box,signal);files=await rawToMzml(wr.files,box);files=await moduloToMzml(files,box,rep);panel=/** @type {FileCarica[] & { panel?: typeof panel }} */ (files).panel||null;for(const [m,n] of wr.errs)(rep.errs[m]=rep.errs[m]||[]).push(n);
  files=files.filter(f=>{const k=fileKind(f.name);if(!k)return true;rejected.push(f);if(k==="other")rep.other.push(f.name);else rep.unopen.push({name:f.name,ext:k});return false});
  // the same group dropped again: no second notice
  if(rep.unopen.length){const h=groupHash(/** @type {FileCarica[]} */ (rep.unopen.map(x=>rejected.find(f=>f.name===x.name))));if(groupSeen(h)){rep.unopen=[];rep.other=[]}else groupMark(h)}
  // all the files of this loading are in the batch before the first one is read: the total only grows in precision, never goes back
  for(const f of files){PROGRESSO.begin(f.name,NATIVO.size(f));CONVROWS.add(f.name,"wait",f.mzlabNat?memDesc(f.mzlabNat.members.map(m=>({name:m.name,size:m.blob.size}))):"");CONVROWS.base(f.name,0)}
  // in the browser the engine builds the indexes of several files at once (pool of workers)
  if(window.QQQ_BROWSER&&files.length>1)await Promise.all(files.map(one));else for(const f of files)await one(f);
  await loneWiffs(wr,rep);if(panel)panel.end();reportLoad(rep)}
function showStart(/** @type {{ files?: FileRiga[], methods?: unknown[] }} */ d){if(typeof AVVIO!=="undefined")AVVIO.rimuoviScheletro();mergeFiles(d.files||[]);ST.methods=d.methods||[];renderMethods();applyView()}
$("#dropdam").onclick=()=>$("#pickdam").click();$("#dropdam").onkeydown=(/** @type {KeyboardEvent} */ e)=>{if(e.key==="Enter"||e.key===" "){e.preventDefault();$("#pickdam").click()}};$("#pickdam").onchange=(/** @type {Event & { target: HTMLInputElement & { files: FileList } }} */ e)=>{upload([...e.target.files],"dam");e.target.value=""};
$("#cartpick").onclick=()=>{if(typeof CartPunt!=="undefined"&&(CartPP.supported()||CartPP.S.native))CartPunt.choose();else $("#pickdir").click()};$("#pickdir").onchange=(/** @type {Event & { target: HTMLInputElement & { files: FileList } }} */ e)=>{const go=flowOpen(e.target),p=CartCarica.fromInput(e.target.files);e.target.value="";if(go)p.then(idle).then(directDone)};
// Flow's «Add files» and «Add a folder» set data-flow-open on the input: the files read are then opened at once, as dropped ones are (Flow reads the open files, not the loading table)
const flowOpen=(/** @type {HTMLElement} */ el)=>{const on=el.dataset.flowOpen==="1";delete el.dataset.flowOpen;return on};
$("#drop").onclick=()=>{delete $("#pick").dataset.flowOpen;$("#pick").click()};$("#pick").onchange=(/** @type {Event & { target: HTMLInputElement & { files: FileList } }} */ e)=>{const go=flowOpen(e.target),p=upload([...e.target.files],"data");e.target.value="";if(go)p.then(idle).then(directDone)};
["dragenter","dragover"].forEach(t=>$("#drop").addEventListener(t,(/** @type {Event} */ e)=>{e.preventDefault();$("#drop").classList.add("over")}));
["dragleave","drop"].forEach(t=>$("#drop").addEventListener(t,(/** @type {Event} */ e)=>{e.preventDefault();$("#drop").classList.remove("over")}));
// a dropped folder is read by cartella-carica.js
$("#drop").addEventListener("drop",(/** @type {DragEvent & { dataTransfer: DataTransfer }} */ e)=>{if(!CartCarica.fromDrop(e))upload([...e.dataTransfer.files],"data")});
["dragenter","dragover"].forEach(t=>$("#dropdam").addEventListener(t,(/** @type {Event} */ e)=>{e.preventDefault();$("#dropdam").classList.add("over")}));
["dragleave","drop"].forEach(t=>$("#dropdam").addEventListener(t,(/** @type {Event} */ e)=>{e.preventDefault();$("#dropdam").classList.remove("over")}));
$("#dropdam").addEventListener("drop",(/** @type {DragEvent & { dataTransfer: DataTransfer }} */ e)=>upload([...e.dataTransfer.files],"dam"));
// a missed drop must not open the file
addEventListener("dragover",e=>e.preventDefault());addEventListener("drop",e=>e.preventDefault());
// The list is opened file by file: the first one at once (the view opens on it), the others as they are read, in the order of the list. The files still to come
// are in E.pending (grey rows, aria-busy, not selectable); a file that fails stays there in red with its message and the others go on; a file taken off the list is not read.
async function exploreList(/** @type {{ file: string, label?: string | null }[]} */ samples){
  if(samples.length<2||!(window.QQQ_BROWSER||/[?&]progressivo=1\b/.test(location.search))){await post("api/explore",{samples});return null}
  await post("api/explore",{samples:[samples[0]],all:samples.map(s=>s.file)});
  const tok=E.progTok={};E.pendN=samples.length;
  E.pending=samples.slice(1).map(s=>({file:s.file,label:s.label||s.file,state:"wait"}));
  return ()=>openRest(samples.slice(1),tok)}
async function openRest(/** @type {{ file: string, label?: string | null }[]} */ rest,/** @type {object} */ tok){
  let last=0,added=/** @type {Pendiente[]} */ ([]);
  const boot=async()=>{last=Date.now();const a=added;added=[];await bootSession();E.pending=E.pending.filter((/** @type {Pendiente} */ x)=>!a.includes(x));renderFileList()};
  for(const s of rest){
    if(E.progTok!==tok)return;
    // taken off the list: not read
    const p=E.pending.find((/** @type {Pendiente} */ x)=>x.file===s.file&&x.state==="wait");if(!p)continue;
    try{await post("api/explore",{samples:[s],append:true})}
    catch(e_){const e=/** @type {Lanciato} */(e_);p.state="err";p.msg=e.message;renderFileList();continue}
    // LR: the overlay is redrawn at most every ~200 ms, whatever the number of files that arrived
    added.push(p);if(Date.now()-last>=200)await boot();
  }
  if(E.progTok===tok){if(added.length)await boot();E.pendN=0;renderFileList();uiSave()}}
// opens the ticked files of the loading table: the first one at once, the others as they are read (E.pending); `scroll` brings the page back to the top (the button of the table)
async function openStaged(/** @type {boolean} */ scroll){
  const rest=await exploreList(ST.files.filter(usable).map((/** @type {FileRiga} */ f)=>({file:f.name,time:f.time,type:f.type,conc:isCal(f)?f.conc:null,cunit:CU[ST.unit][0],label:(E.files.find((/** @type {FileAperto} */ x)=>x.file===f.name)||{}).label})));
  STATO.aggiunta(false,"adding");await bootSession();RUOLI.commit(ST.files);applyView();if(scroll)window.scrollTo(0,0);
  await Promise.all(E.panels.map((/** @type {{ ready?: unknown }} */ p)=>p.ready).filter(Boolean)).catch(()=>{});uiSave();if(rest)rest()}
// Files dropped on the side panel of the data view are added at once: the same upload() and the same opening as the loading table, without the loading screen (the progress is the bar of the list, E.pending);
// files of the other resolution than the open ones stay in the loading table, unticked, and a note says so
const DIRECT={tm:0};
const dropReady=()=>!!(S.sess&&!S.adding&&!$("#v-data").hidden);
const dropHasFiles=(/** @type {DragEvent & { dataTransfer: DataTransfer }} */ e)=>[...((e.dataTransfer&&e.dataTransfer.types)||[])].includes("Files");
// a clean «added» is only announced and then fades away; warnings and failures stay 9 s
/** @param {unknown} [p] */ function directNote(/** @type {string} */ key,p){const n=$("#fl-note");clearTimeout(DIRECT.tm);n.textContent=key?I18N.t(key,p):"";n.hidden=!key;n.style.opacity="";
  if(key==="files.drop.done"){DIRECT.tm=setTimeout(()=>{n.style.opacity="0";DIRECT.tm=setTimeout(()=>directNote(""),600)},900)}else if(key)DIRECT.tm=setTimeout(()=>directNote(""),9000)}
function dropZone(/** @type {boolean} */ on){$("#sb-pane").classList.toggle("dz-over",on);$("#fl-dz").hidden=!on}
async function directDone(){
  const open=(/** @type {FileRiga} */ f)=>E.files.some((/** @type {FileAperto} */ x)=>!x.gone&&(x.file===f.name||x.file.startsWith(f.name+"#")));
  ST.files.forEach((/** @type {FileRiga} */ f)=>{if(open(f))f.use=true});
  const fresh=ST.files.filter((/** @type {FileRiga} */ f)=>f.kind in GORD&&!open(f));
  if(!fresh.length){directNote("");return}
  const ref=ST.files.find(open),world=ref?!!ref.hr:fresh.filter((/** @type {FileRiga} */ f)=>f.hr).length>fresh.filter((/** @type {FileRiga} */ f)=>!f.hr).length;
  fresh.forEach((/** @type {FileRiga} */ f)=>{f.use=!!f.hr===world});
  const skipped=fresh.filter((/** @type {FileRiga} */ f)=>!f.use).length,n=fresh.length-skipped;
  try{if(n)await openStaged(false);directNote(skipped?"files.drop.mixed":"files.drop.done",{n,m:skipped})}
  catch(e_){const e=/** @type {Lanciato} */(e_);directNote("files.drop.fail",{msg:e.message})}
  renderFiles()}
(()=>{const dz=$("#dfiles");
  ["dragenter","dragover"].forEach(t=>dz.addEventListener(t,(/** @type {DragEvent & { dataTransfer: DataTransfer }} */ e)=>{if(!dropReady()||!dropHasFiles(e))return;e.preventDefault();dropZone(true)}));
  dz.addEventListener("dragleave",(/** @type {DragEvent & { dataTransfer: DataTransfer }} */ e)=>{if(!dz.contains(e.relatedTarget))dropZone(false)});
  dz.addEventListener("drop",(/** @type {DragEvent & { dataTransfer: DataTransfer }} */ e)=>{dropZone(false);if(!dropReady()||!dropHasFiles(e))return;e.preventDefault();e.stopPropagation();
    const fs=[...e.dataTransfer.files],done=()=>{if(!UPBUSY&&dropReady())directDone()};directNote("files.drop.adding");
    if(!CartCarica.fromDrop(e,done))upload(fs,"data").then(done)});
  $("#addf").addEventListener("click",()=>directNote(""))})();
$("#hrmix-lr").onclick=()=>{hrMixPick(false);renderFiles()};$("#hrmix-hr").onclick=()=>{hrMixPick(true);renderFiles()};
$("#opbtn").onclick=async()=>{const b=$("#opbtn");b.disabled=true;loading(true);
  try{if(UPBUSY>0){WAITING=true;WAIT0=Date.now();waitStato();const tk=setInterval(waitStato,1000);ldAction(I18N.t("conv.cancel"),cancelLoads);try{await idle()}finally{clearInterval(tk);ldAction("");WAITING=false;loadingStato(null)}}
    if(!ST.files.some(usable)){checkGo();return}
    // the time of a dark is negative whatever way it got here (name, table, old project)
    ST.files.forEach((/** @type {FileRiga} */ f)=>{if(RUOLI.roleOf(f)==="dark")f.time=darkTime(f.time)});
    await openStaged(true)}
  catch(e_){const e=/** @type {Lanciato} */(e_);
    if(e.key==="err.hr.mix"){
      $("#gomsg").innerHTML=`<span class="fail">${I18N.t("err.hr.mix")}</span> 
      <button id="hrmix-replace" class="btn">${I18N.t("load.hrmix.replace")}</button> 
      <button id="hrmix-cancel" class="btn">${I18N.t("common.cancel")}</button>`;
      loading(false);
      const isOpen=(/** @type {FileRiga} */ f)=>typeof E!=="undefined"&&E.files.some((/** @type {FileAperto} */ x)=>x.file===f.name||x.file.startsWith(f.name+"#"));
      $("#hrmix-cancel").onclick=()=>$("#cancadd").click();
      $("#hrmix-replace").onclick=async()=>{
        const b=$("#hrmix-replace");b.disabled=true;loading(true);
        try {
          // closes the open files but keeps the uploaded ones on disk
          await post("api/new",{});
          await post("api/explore",{samples:ST.files.filter((/** @type {FileRiga} */ f)=>usable(f)&&!isOpen(f)).map((/** @type {FileRiga} */ f)=>({file:f.name,time:f.time,type:f.type,conc:isCal(f)?f.conc:null,cunit:CU[ST.unit][0],label:null}))});
          STATO.aggiunta(false,"adding");await bootSession();RUOLI.commit(ST.files);applyView();await Promise.all(E.panels.map((/** @type {{ ready?: unknown }} */ p)=>p.ready).filter(Boolean)).catch(()=>{});uiSave();
        }catch(e2_){const e2=/** @type {Lanciato} */(e2_);$("#gomsg").innerHTML=h`<span class="fail">${e2.message}</span>`;b.disabled=false}finally{loading(false)}
      };
    } else {
      $("#gomsg").innerHTML=h`<span class="fail">${e.message}</span>`;
    }
    b.disabled=false
  }finally{if($("#gomsg").innerHTML==="")loading(false)}
};
// both exits of the loading screen (Cancel, «Back to the data») give the page back as it was: the side panel returns through BARRA, the scroll here
function leaveAdding(){const y=_addY;STATO.aggiunta(false,"adding");applyView();// after the charts took their size again (the tpview handler of the data view, one timeout earlier), or the page is still too short to reach y
  if(y!=null)setTimeout(()=>window.scrollTo(0,y),0)}
$("#cancadd").onclick=leaveAdding;
// while adding files the ones already in the session stay visible, with a way back to the data
function renderLoaded(){const el=$("#loaded"),on=!!(S.sess&&S.adding&&typeof E!=="undefined"&&E.files.some((/** @type {FileAperto} */ f)=>!f.gone));el.hidden=!on;if(!on)return;
  const fs=E.files.filter((/** @type {FileAperto} */ f)=>!f.gone);
  el.innerHTML=`<div class="row" style="display:flex;gap:10px;align-items:center;flex-wrap:wrap"><b>${I18N.t("load.loaded.title",{n:fs.length})}</b><span class="sp" style="flex:1"></span><button class="sec" id="clearall" type="button" title="${I18N.t("load.loaded.clearTip")}">${I18N.t("load.loaded.clear")}</button><button class="go" id="backdata">${I18N.t("load.loaded.back")}</button></div><div class="muted sm" style="margin-top:6px">${fs.map((/** @type {FileAperto} */ f)=>`<span style="white-space:nowrap;margin-right:12px"><span style="display:inline-block;width:9px;height:9px;border-radius:2px;background:${f.color||"#888"};margin-right:4px"></span>${esc(f.label||f.file)}</span>`).join("")}</div>`;
  $("#backdata").onclick=leaveAdding;
  // two presses (the first asks): the same clean slate as «Files and session only» of «Start a new session?» (reset.js)
  const ca=$("#clearall");let armed=0;ca.onclick=()=>{if(!armed){armed=setTimeout(()=>{armed=0;ca.textContent=I18N.t("load.loaded.clear")},6000);ca.textContent=I18N.t("load.loaded.clearSure",{n:fs.length});return}
    clearTimeout(armed);ca.disabled=true;ca.textContent=I18N.t("load.loaded.clearBusy");
    // a failure gives the button back with a clear message: it is never left busy
    Promise.resolve(QQRESET.run("files")).catch(()=>{ca.disabled=false;ca.textContent=I18N.t("load.loaded.clearFail")})}}
// New session: QQRESET.ask (reset.js) asks once, then clears everything the app keeps in this browser and reloads
$("#newrun").onclick=()=>QQRESET.ask();
// "m/z" is always written in italics: any text node that contains it is split and the symbol wrapped in <i> (not inside inputs, options, scripts or an existing <i>)
(function(){const SKIP=/^(SCRIPT|STYLE|OPTION|TEXTAREA|INPUT|CANVAS|TITLE|SELECT)$/,RX=/(m\/z|MS2)/;
 // m/z always in italics and MS2 with a real superscript (never the Unicode characters); text nodes only: titles, options and file names keep the plain "MS2"
 function fix(/** @type {Node} */ root){const w=document.createTreeWalker(root,NodeFilter.SHOW_TEXT,{acceptNode:n=>RX.test(/** @type {string} */ (n.nodeValue))&&n.parentElement&&!SKIP.test(n.parentElement.tagName)&&!(n.parentElement.tagName==="I"&&n.nodeValue==="m/z")&&!(n.parentElement.tagName==="SUP")?NodeFilter.FILTER_ACCEPT:NodeFilter.FILTER_REJECT});
  const L=[];while(w.nextNode())L.push(w.currentNode);
  for(const n of /** @type {any[]} */ (L)){const f=document.createDocumentFragment();n.nodeValue.split(/(m\/z|MS2)/).forEach((/** @type {string} */ t)=>{if(t==="m/z"){const e=document.createElement("i");e.textContent="m/z";f.appendChild(e)}else if(t==="MS2"){f.appendChild(document.createTextNode("MS"));const e=document.createElement("sup");e.textContent="2";f.appendChild(e)}else if(t)f.appendChild(document.createTextNode(t))});n.replaceWith(f)}}
 let busy=false;new MutationObserver(ms=>{if(busy)return;busy=true;try{for(const m of ms){if(m.type==="characterData"){if(m.target.parentElement)fix(m.target.parentElement)}else m.addedNodes.forEach(a=>{if(a.nodeType===1)fix(a);else if(a.nodeType===3&&a.parentElement)fix(a.parentElement)})}}finally{busy=false}}).observe(document.body,{childList:true,subtree:true,characterData:true});
 fix(document.body)})();
document.querySelectorAll("[data-ic]").forEach(e=>{e.innerHTML=QICON.get(e.dataset.ic,16);
  // the header icons sit next to their text label (the icon names are Italian)
  if(e.closest("header"))e.querySelectorAll("svg").forEach((/** @type {Element} */ v)=>{v.removeAttribute("aria-label");v.removeAttribute("role");v.setAttribute("aria-hidden","true")})});
// What is downloaded when (decided 8/10/2026, measured on the site): the engine (~10 MB compressed) at once, because Dati needs it;
// after it, one step per idle moment: OpenChemLib ~340 kB, then the Teoria ~60 kB; Ketcher is warmed by first-paint.js (one policy: low priority, once loaded and no file loading,
// not with data saving on) or at once when the mouse goes on «Disegno»; the example files (~28 MB) only with their button.
/** @param {string} [as] */ function prefetch(/** @type {string[]} */ urls,as){urls.forEach(u=>{const l=document.createElement("link");l.rel=as==="module"?"modulepreload":"prefetch";l.href=u;if(as&&as!=="module")l.as=as;document.head.appendChild(l)})}
function prefetchIdle(){const idle=/** @type {any} */ (window).requestIdleCallback;
  const c=/** @type {any} */ (navigator).connection||{};if(c.saveData)return;
  // one step per idle moment, in this order: Disegno (OpenChemLib), then the Teoria (the Dati scripts are all in carica.js's list; the other language is fetched only when chosen)
  const steps=[()=>prefetch(["static/vendor/openchemlib.js"],"module"),
    ()=>{if(SITO_PUBBLICO.teoria)prefetch(["static/teoria/index.html","static/teoria/teoria.css","static/teoria/teoria.js","static/teoria/tema.js","static/teoria/glossario-dati.js"])}];
  const next=()=>{const f=steps.shift();if(!f)return;f();idle?idle(next,{timeout:8000}):setTimeout(next,2000)};
  idle?idle(next,{timeout:8000}):setTimeout(next,2000)}
{const b=document.querySelector('#nav button[data-v="draw"]');if(b){const warm=()=>{const w=/** @type {any} */ (window).MZLAB_WARM_KETCHER;if(w)w()};b.addEventListener("pointerenter",warm,{once:true});b.addEventListener("focus",warm,{once:true})}}
// draw.js (Disegno, with OpenChemLib) is not even fetched on a phone
{const s=document.createElement("script");s.type="module";s.src="static/draw.js";document.body.appendChild(s);init()}
// Five logo hits turn the administrator mode on without a reload: the Teoria already open in the frame reloads, so its editor button and Alt+E appear
document.addEventListener("mzlab:admin",()=>{const f=$("#tframe");if(f&&f.getAttribute("src")&&f.contentWindow)try{f.contentWindow.location.reload()}catch(_){}});
