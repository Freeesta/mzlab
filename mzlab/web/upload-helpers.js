// @ts-strict
// @ts-noany
// Upload helpers split out of shell.js (classic script, shared globals): file kind, sessionStorage-backed "seen groups", duplicate filter.
function fileKind(/** @type {string} */ name){const n=name.toLowerCase();if(/\.(mzml|dam)$/.test(n))return null;const M=window.LETTORI_MOD;if(M&&M.extensions().some((/** @type {string} */ e)=>n.endsWith(e)))return null;return window.FORMATI_NOTI.classify(n)||"other"}
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
function dropDups(/** @type {FileCarica[]} */ files){
  // a file of the table that is not open (removed, set aside) is not a duplicate: picking it again ticks it, as its box in the table would
  const row=(/** @type {FileCarica} */ f)=>ST.files.find((/** @type {FileRiga} */ g)=>g.name===f.name&&g.size===f.size),again=files.filter(f=>{const g=row(f);return !INFLIGHT.has(fkey(f))&&g&&g.kind in GORD&&!OPENNOW.isOpen(g.name)});
  if(again.length){again.forEach(f=>OPENNOW.tick(row(f),true));renderFiles();files=files.filter(f=>!again.includes(f))}
  const dups=files.filter(f=>INFLIGHT.has(fkey(f))||ST.files.some((/** @type {FileRiga} */ g)=>g.name===f.name&&g.size===f.size));if(!dups.length)return files;
  const msg=dups.length===1?I18N.t("load.dup.one",{name:dups[0].name}):I18N.t("load.dup.many",{n:dups.length});CartCarica.note(msg);setTimeout(()=>{const n=$("#cartinfo");if(n&&n.textContent===msg)CartCarica.note("")},5000);
  dups.forEach(f=>{const tr=[...document.querySelectorAll("#flist tr")].find(r=>r.querySelector(".ldname")?.getAttribute("title")===f.name);if(tr){tr.classList.add("dupflash");tr.scrollIntoView({block:"nearest"});setTimeout(()=>tr.classList.remove("dupflash"),1800)}});
  return files.filter(f=>!dups.includes(f))}
// the members of a file without their names, for the diagnosis: ".wiff 1.2 MB + .wiff.scan 57.0 MB"
const memDesc=(/** @type {{ name: string, size: number }[]} */ fs)=>fs.map(b=>`${(String(b.name).match(/\.[^./\\]+(\.scan)?$/i)||["?"])[0]} ${(b.size/1048576).toFixed(1)} MB`).join(" + ");
