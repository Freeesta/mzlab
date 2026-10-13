// Model of the Dati view for tsc (job «tipi»), used in JSDoc as {MzFile}, {Pannello}, {Sessione}, {Preferenze} (names that do not clash with lib.dom).
// Types only: nothing here is loaded by the page. The saved shape of the notebook (NB.ui in explore.js uiSave) does not change.
// The experiment types are the ids of the registry ESP (esperimenti.js, format esperimenti/1), not a list copied here: Tab is a string.

/** Id of an experiment type in the registry ESP.ids (E.tab, p.tab). */
type Scheda = string;

/** One open file of the session (E.files[k]): the object of api/session plus the fields of the page. */
interface MzFile {
  file: string;            // name of the file (key everywhere: notebook, panels, lists)
  k: number;               // index in E.files
  label: string;
  time: number | null;     // minutes
  type: string;
  kind: Scheda;            // «full» / «ms2» / «mrm»: the experiment of the file
  lv: number;              // MS level shown by default (2 for ms2)
  vis: boolean;            // shown in the graphs
  color?: string; colorSet?: boolean;
  conc?: number | null; cunit?: string | null; rep?: number;
  gone?: boolean;          // removed from the session, still listed until the next reload
  rt_min?: number; rt_max?: number; ms1?: number; ms2?: number;
  hr?: boolean; dda?: boolean;
  [campo: string]: any;    // the rest of api/session (profiles, MRM transitions, ...)
}

/** A trace of an XIC panel. */
interface Traccia { id: number; mz: number; w: number | null; label: string; ion?: boolean; obs?: boolean; [campo: string]: any }

/** Fields every panel has. The persistent ones are saved by uiSave (explore.js) into NB.ui.panels; the others live only in the page. */
interface PannelloBase {
  // persistent (uiSave)
  id: number;
  type: "chrom" | "spec" | "xic" | "mrm" | "map";
  tab: Scheda;
  title: string;
  x: number; y: number; w: number; h: number; full?: boolean;
  zoom: [number, number] | null;
  anns: any[]; ints: any[];
  k?: number;               // index of the file in E.files (saved as the file name)
  link?: number | null; src?: number | null;   // id of another panel (saved as its index in E.panels)
  // runtime only
  el: HTMLElement;          // the card of the panel
  sel: any; cur: any;       // selection and cursor of the chart
  _a?: any;                 // axes and geometry of the last draw
  [campo: string]: any;
}
interface PannelloChrom extends PannelloBase { type: "chrom"; kind: string; prec: number | null; smooth: boolean; traces: Traccia[] }
interface PannelloXic extends PannelloBase { type: "xic"; traces: Traccia[]; tol: number; smooth: boolean; adduct: any }
interface PannelloMrm extends PannelloBase { type: "mrm"; tr: string; smooth: false; _trs: any[] }
interface PannelloSpec extends PannelloBase { type: "spec"; k: number; all: boolean; r0: number | null; r1: number | null; level: number; prec: number | null; lock: any; rel: boolean | null }
interface PannelloMappa extends PannelloBase { type: "map"; k: number; scale: string; norm: string }
/** A panel is a view (type) on an experiment (tab): the union is discriminated by `type`. */
type Pannello = PannelloChrom | PannelloXic | PannelloMrm | PannelloSpec | PannelloMappa;

/** NB.session[i]: what the notebook remembers of a file. */
interface FileSalvato { file: string; label: string; time: number | null; type: string; conc: number | null; cunit: string | null; rep?: number }

/** The session of api/session (S.sess): the list of the open files as the engine sends it. */
type Sessione = Array<Partial<MzFile> & { file: string }> | null;

/** UIP (explore.js, saved in localStorage «qqq.prefs» by settings.js uipSave). */
interface Preferenze { theme: "auto" | "light" | "dark"; pal: string; merge: boolean; hrPpm: number; hrDec: number; tog: boolean }
