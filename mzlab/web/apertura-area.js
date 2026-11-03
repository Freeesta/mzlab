// @ts-strict
// @ts-noany
// Skeleton #opening inside #v-data while the ticked files open: title, phase, progress and file list, drawn by openingRender and shown or hidden by applyView.
// It is driven by avvio-progressivo.js and area-lavoro.js; on the way out the work area fades in once (class opn-rev).
const OPEN={files:/** @type {{ label: string, st: string }[]} */([]),msg:"",on:false,tm:0,pct:/** @type {number | null} */(null)};
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
