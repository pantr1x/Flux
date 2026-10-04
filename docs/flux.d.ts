// Types of the `flux` object that Flux Native gives your plugin.
// Put this file next to your plugin and add `/** @param {import('./flux').Flux} flux */` above activate()
// so VS Code (or Flux) can show you the functions while you type.

export interface FluxFile {
  path: string;
  name: string;
  /** "python", "javascript", "typescript", "html", "css", "c", "cpp", "java", "csharp", "go", "rust", … */
  language: string;
  text: string;
  /** character offsets; start === end when nothing is selected */
  selection: { start: number; end: number; text: string };
  /** 1-based, like the status bar */
  cursor: { line: number; column: number };
}

export interface StatusItem {
  set(text: string): void;
  setTitle(text: string): void;
  show(visible: boolean): void;
  remove(): void;
}

export interface Mark {
  /** 0-based line */
  line: number;
  /** 0-based column (default 0) */
  col?: number;
  /** characters to underline (0 = to the end of the line) */
  len?: number;
  /** "#rrggbb" – default amber, or red with kind: "error" */
  color?: string;
  kind?: "hint" | "error";
  /** short text shown after the line and on hover */
  message?: string;
}

export interface ThemeColors {
  fg?: string; comment?: string; keyword?: string; storage?: string; string?: string; number?: string;
  type?: string; function?: string; variable?: string; parameter?: string; property?: string;
  constant?: string; tag?: string; attr?: string; delimiter?: string; regexp?: string;
}

export interface Flux {
  /** your plugin id from plugin.json */
  id: string;
  /** "native-1" in Flux Native */
  version: string;
  toast(text: string, kind?: "info" | "ok" | "warn" | "error"): void;
  activeFile(): FluxFile | null;
  /** inserts at the cursor; $0 / $1 / ${1:text} place the cursor like snippets */
  insertText(text: string): void;
  replaceSelection(text: string): void;
  commands: {
    /** adds a command to Ctrl+Shift+A; key like "Ctrl+Alt+H" */
    register(id: string, label: string, run: () => void, options?: { key?: string }): void;
    /** your command, or a built-in one: "run", "save", "settings", "new-file", "open-folder", "ai", … */
    run(id: string): void;
  };
  statusBar: { add(item: { text: string; title?: string; onClick?: () => void }): StatusItem };
  snippets: {
    /** language like activeFile().language, or "*" for all; body uses $1, ${1:name}, $0 */
    add(language: string, prefix: string, body: string, description?: string): void;
  };
  themes: { add(key: string, theme: { name: string; type: "dark" | "light"; colors: ThemeColors; basedOn?: string }): void };
  marks: { set(list: Mark[]): void; clear(): void };
  onOpen(cb: (file: FluxFile | null) => void): void;
  onSave(cb: (file: FluxFile | null) => void): void;
  /** called 300 ms after typing stops */
  onChange(cb: (file: FluxFile | null) => void): void;
  onSelection(cb: (file: FluxFile | null) => void): void;
  /** like setInterval, at least 250 ms */
  every(ms: number, cb: () => void): void;
  project: {
    folder(): string | null;
    /** paths relative to the project, with "/" */
    files(): string[];
    /** text of a file inside the project (throws if it can't be read) */
    read(path: string): string;
  };
  storage: {
    get<T = unknown>(key: string, fallback?: T): T;
    set(key: string, value: unknown): void;
  };
}
