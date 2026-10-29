// @ts-strict
// @ts-noany
// Work area while the ticked files are opened: the skeleton #opening inside #v-data (title, phase, progress, files, Cancel). Used by the «Load data» handler and
// openStaged (shell.js) and by applyView; marks for the ?perf meter and MZLAB_OPEN (time to work area, to first file read, to first file shown).
// The click draws the work area at once (applyView with S.opening) and the loading state sits inside it: the ticked files, the phase and, while copies or indexing go on, the
// progress of the whole batch with Cancel. The full-screen veil is kept only when there is nothing usable yet to open (files still being copied) and as the error exit:
// opening ends (STATO.apertura(false)), the veil goes down, the table is back with the red message and the button is live again.
const OPEN={files:/** @type {{ label: string, st: string }[]} */([]),msg:"",on:false,tm:0,pct:/** @type {number | null} */(null),t:/** @type {Record<string, number>} */({})};
const openMark=(/** @type {string} */ k)=>{const t=performance.now();OPEN.t[k]=t;try{performance.mark("mzlab-open:"+k)}catch(e){}if(window.PERF)PERF.mark("open: "+k)};
/** @type {any} */ (window).MZLAB_OPEN=OPEN;
const OPEN_ST=/** @type {Record<string, string>} */ ({wait:"open.st.wait",read:"open.st.read",boot:"open.st.boot",err:"open.st.err"});
function openingRender(){const el=$("#opening");if(el.hidden)return;
  const first=OPEN.files[0];$("#opn-title").textContent=first?I18N.t("open.title",{file:first.label}):"";
  $("#opn-msg").textContent=OPEN.msg;const bar=$("#opn-bar");bar.hidden=OPEN.pct==null;if(OPEN.pct!=null){bar.setAttribute("role","progressbar");bar.setAttribute("aria-valuenow",String(OPEN.pct));bar.firstElementChild.style.width=OPEN.pct+"%"}
  $("#opn-list").innerHTML=OPEN.files.map(f=>h`<li class="${f.st}"><span class="nm">${f.label}</span> <small>${I18N.t(OPEN_ST[f.st]||"open.st.wait")}</small></li>`).join("")}
// the skeleton is shown or hidden by applyView; the tools and the file list wait for the session (inert), the charts stay hidden. #dpanels keeps its place (class opn: invisible, not
// display:none) so the panels are built at their final size and nothing moves when they replace the skeleton, which fades in the content once (class opn-rev, opacity only)
function openingView(/** @type {boolean} */ on){const v=$("#v-data");$("#opening").hidden=!on;v.classList.toggle("opn",on);
  for(const id of ["#tools","#dfiles"])$(id).inert=on;v.setAttribute("aria-busy",String(on));if(on)openingRender();
  else if(OPEN.on&&!v.hidden){v.classList.add("opn-rev");clearTimeout(OPEN.tm);OPEN.tm=setTimeout(()=>v.classList.remove("opn-rev"),400)}
  OPEN.on=on}
function openingStato(/** @type {string} */ msg,/** @type {number | null} */ pct){OPEN.msg=msg;OPEN.pct=pct;openingRender()}
function openingFile(/** @type {number} */ i,/** @type {string} */ st){if(OPEN.files[i])OPEN.files[i].st=st;openingRender()}
function openingFiles(){OPEN.files=ST.files.filter(usable).map((/** @type {FileRiga} */ f,/** @type {number} */ i)=>({label:f.name,st:i?"wait":"read"}))}
function openingStart(){openingFiles();OPEN.msg="";OPEN.pct=null;const a=$("#opn-act");a.textContent="";a.hidden=true;
  STATO.apertura(true,"opening");applyView();openMark("area");$("#opening").focus();
  // a veil raised by someone else (the tour's example loader) or by this click while nothing was usable comes down: the work area replaces it
  if(!$("#loading").hidden)loading(false)}
function openingEnd(){if(!S.opening)return;STATO.apertura(false,"opening");applyView()}
// Cancel of the copies under way: a button inside the skeleton, or the action of the veil when that is what is up
function openingCancel(/** @type {() => void} */ fn){if(!S.opening){ldAction(I18N.t("conv.cancel"),fn);return}
  const a=$("#opn-act"),c=document.createElement("button");c.type="button";c.className="btn imp";c.textContent=I18N.t("conv.cancel");c.onclick=fn;a.textContent="";a.hidden=false;a.appendChild(c)}
