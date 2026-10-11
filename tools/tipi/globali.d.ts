// Globals shared by the classic scripts of the page (AGENTS.md §4), declared here for tsc only (job «tipi» of test.yml).
// Add a line when a file with `// @ts-check` uses a global defined in another file.
interface Window {
  APP_NAME: string;
  MZLAB_VERSION: string;
  MZLAB_COMMIT: string;
  MZLAB_PHONE: boolean;
  QQQ_PHONE: boolean;
  FEATURES: any;
  MIRINO: any;
  QUALITA: any;
  STAT: any;
  HR: any;
}
declare const BANCO: any;
declare const APP_NAME: string;
declare const I18N: { t(key: string, params?: Record<string, unknown>): string; [k: string]: any };
declare function dl(name: string, text: string | Uint8Array, type?: string): void;
