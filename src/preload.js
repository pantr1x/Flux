// Bezpečný most medzi oknom (UI) a hlavným procesom – UI nemá priamy prístup k Node.js.
const { contextBridge, ipcRenderer } = require('electron');

const on = (channel) => (callback) => {
  const listener = (_e, payload) => callback(payload);
  ipcRenderer.on(channel, listener);
  return () => ipcRenderer.removeListener(channel, listener);
};

contextBridge.exposeInMainWorld('flux', {
  init: () => ipcRenderer.invoke('app:init'),
  setSettings: (patch) => ipcRenderer.invoke('app:set-settings', patch),
  setDirty: (count) => ipcRenderer.send('app:dirty', count),
  close: () => ipcRenderer.send('app:close'),
  onSaveAllAndClose: on('app:save-all-and-close'),

  openFolderDialog: () => ipcRenderer.invoke('workspace:open-dialog'),
  openFolder: (dir) => ipcRenderer.invoke('workspace:open', dir),
  projects: () => ipcRenderer.invoke('workspace:projects'),
  forgetProject: (dir) => ipcRenderer.invoke('workspace:forget', dir),
  pinProject: (dir, pinned) => ipcRenderer.invoke('project:pin', dir, pinned),
  renameProject: (dir, name) => ipcRenderer.invoke('project:rename', dir, name),
  projectRoot: () => ipcRenderer.invoke('project:root'),
  createProject: (name, root) => ipcRenderer.invoke('project:create', name, root),
  chooseProjectRoot: () => ipcRenderer.invoke('project:choose-root'),
  readImage: (file) => ipcRenderer.invoke('fs:read-image', file),

  list: (dir) => ipcRenderer.invoke('fs:list', dir),
  listAll: () => ipcRenderer.invoke('fs:list-all'),
  read: (file) => ipcRenderer.invoke('fs:read', file),
  readAny: (file) => ipcRenderer.invoke('fs:read-any', file),
  write: (file, content) => ipcRenderer.invoke('fs:write', file, content),
  create: (target, isDir) => ipcRenderer.invoke('fs:create', target, isDir),
  rename: (from, to) => ipcRenderer.invoke('fs:rename', from, to),
  trash: (target) => ipcRenderer.invoke('fs:trash', target),
  reveal: (target) => ipcRenderer.invoke('fs:reveal', target),
  onFsChanged: on('fs:changed'),
  treeMenu: (item) => ipcRenderer.invoke('menu:tree', item),

  findPython: () => ipcRenderer.invoke('python:find'),
  choosePython: () => ipcRenderer.invoke('python:choose'),
  resetPython: () => ipcRenderer.invoke('python:reset'),

  runFile: (file, python, lang) => ipcRenderer.invoke('run:file', file, python, lang),
  pipInstall: (python, pkg) => ipcRenderer.invoke('run:pip', python, pkg),
  createVenv: (python) => ipcRenderer.invoke('run:create-venv', python),
  input: (text) => ipcRenderer.send('run:input', text),
  stop: () => ipcRenderer.send('run:stop'),
  resize: (cols, rows) => ipcRenderer.send('run:resize', cols, rows),
  onRunStart: on('run:start'),
  onRunData: on('run:data'),
  onRunExit: on('run:exit'),

  liveStart: () => ipcRenderer.invoke('live:start'),
  liveStop: () => ipcRenderer.invoke('live:stop'),
  onLiveLog: on('live:log'),
  onLiveStopped: on('live:stopped'),
  openExternal: (url) => ipcRenderer.invoke('shell:open-external', url),

  lspStart: () => ipcRenderer.invoke('lsp:start'),
  lspSend: (msg) => ipcRenderer.send('lsp:send', msg),
  onLspMessage: on('lsp:message'),
  onLspExit: on('lsp:exit'),
});
