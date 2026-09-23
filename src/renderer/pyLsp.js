// Minimálny LSP klient: Pyright → Monaco (autocomplete, nápoveda, chyby, prejsť na definíciu).
const flux = window.flux;

// LSP CompletionItemKind (1–25) → Monaco CompletionItemKind
function completionKind(monaco, kind) {
  const K = monaco.languages.CompletionItemKind;
  const map = {
    1: K.Text, 2: K.Method, 3: K.Function, 4: K.Constructor, 5: K.Field, 6: K.Variable, 7: K.Class,
    8: K.Interface, 9: K.Module, 10: K.Property, 11: K.Unit, 12: K.Value, 13: K.Enum, 14: K.Keyword,
    15: K.Snippet, 16: K.Color, 17: K.File, 18: K.Reference, 19: K.Folder, 20: K.EnumMember,
    21: K.Constant, 22: K.Struct, 23: K.Event, 24: K.Operator, 25: K.TypeParameter,
  };
  return map[kind] ?? K.Text;
}

const toRange = (monaco, r) =>
  new monaco.Range(r.start.line + 1, r.start.character + 1, r.end.line + 1, r.end.character + 1);

function markup(content) {
  if (!content) return undefined;
  if (typeof content === 'string') return { value: content };
  if (content.kind === 'plaintext') return { value: content.value.replace(/[\\`*_{}[\]()#+\-.!<>]/g, '\\$&') };
  if (content.language) return { value: '```' + content.language + '\n' + content.value + '\n```' };
  return { value: content.value };
}

export class PythonLanguageClient {
  constructor(monaco, { isLibrary, onStatus }) {
    this.monaco = monaco;
    this.isLibrary = isLibrary; // (uri) => true pre súbory mimo projektu (iba na čítanie)
    this.onStatus = onStatus;
    this.nextId = 1;
    this.pending = new Map();
    this.ready = false;
    this.open = new Set();
    this.root = null;
    this.pythonPath = null;
    this.generation = 0;

    flux.onLspMessage((msg) => this.receive(msg));
    flux.onLspExit(() => {
      this.ready = false;
      this.onStatus('error');
    });
    this.registerProviders();

    monaco.editor.onDidCreateModel((model) => this.track(model));
    for (const model of monaco.editor.getModels()) this.track(model);
  }

  // ---------- spojenie ----------
  async start(root, pythonPath) {
    const generation = ++this.generation;
    this.ready = false;
    this.root = root;
    this.pythonPath = pythonPath;
    for (const { reject } of this.pending.values()) reject(new Error('restart'));
    this.pending.clear();
    this.open.clear();
    this.onStatus('starting');
    await flux.lspStart();
    const rootUri = root ? this.monaco.Uri.file(root).toString() : null;
    try {
      await this.request('initialize', {
        processId: null,
        clientInfo: { name: 'Flux' },
        rootUri,
        workspaceFolders: rootUri ? [{ uri: rootUri, name: root.split(/[\\/]/).pop() }] : null,
        capabilities: {
          textDocument: {
            synchronization: { didSave: true, dynamicRegistration: false },
            completion: {
              contextSupport: true,
              completionItem: {
                snippetSupport: true,
                documentationFormat: ['markdown', 'plaintext'],
                labelDetailsSupport: true,
                resolveSupport: { properties: ['documentation', 'detail', 'additionalTextEdits'] },
                tagSupport: { valueSet: [1] },
              },
            },
            hover: { contentFormat: ['markdown', 'plaintext'] },
            signatureHelp: {
              signatureInformation: {
                documentationFormat: ['markdown', 'plaintext'],
                parameterInformation: { labelOffsetSupport: true },
                activeParameterSupport: true,
              },
            },
            definition: { linkSupport: true },
            publishDiagnostics: { tagSupport: { valueSet: [1, 2] } },
          },
          workspace: { configuration: true, workspaceFolders: true, didChangeConfiguration: { dynamicRegistration: true } },
          window: { workDoneProgress: false },
        },
      });
    } catch {
      if (generation === this.generation) this.onStatus('error');
      return;
    }
    if (generation !== this.generation) return;
    this.notify('initialized', {});
    this.notify('workspace/didChangeConfiguration', { settings: {} });
    this.ready = true;
    this.onStatus('ready');
    for (const model of this.monaco.editor.getModels()) this.didOpen(model);
  }

  setPython(pythonPath) {
    this.pythonPath = pythonPath;
    if (this.ready) this.notify('workspace/didChangeConfiguration', { settings: {} });
  }

  request(method, params) {
    const id = this.nextId++;
    flux.lspSend({ jsonrpc: '2.0', id, method, params });
    return new Promise((resolve, reject) => this.pending.set(id, { resolve, reject }));
  }

  notify(method, params) {
    flux.lspSend({ jsonrpc: '2.0', method, params });
  }

  receive(msg) {
    if (msg.id !== undefined && !msg.method) {
      const p = this.pending.get(msg.id);
      if (!p) return;
      this.pending.delete(msg.id);
      if (msg.error) p.reject(new Error(msg.error.message));
      else p.resolve(msg.result);
      return;
    }
    if (msg.id !== undefined && msg.method) {
      flux.lspSend({ jsonrpc: '2.0', id: msg.id, result: this.handleServerRequest(msg) });
      return;
    }
    if (msg.method === 'textDocument/publishDiagnostics') this.diagnostics(msg.params);
  }

  handleServerRequest(msg) {
    if (msg.method === 'workspace/configuration') {
      const analysis = {
        typeCheckingMode: 'basic',
        diagnosticMode: 'openFilesOnly',
        autoImportCompletions: true,
        useLibraryCodeForTypes: true,
        autoSearchPaths: true,
      };
      return msg.params.items.map(({ section }) => {
        if (section === 'python') return { pythonPath: this.pythonPath || undefined, analysis };
        if (section === 'python.analysis') return analysis;
        return {};
      });
    }
    if (msg.method === 'workspace/workspaceFolders') {
      return this.root ? [{ uri: this.monaco.Uri.file(this.root).toString(), name: this.root.split(/[\\/]/).pop() }] : [];
    }
    return null;
  }

  // ---------- synchronizácia dokumentov ----------
  track(model) {
    if (model.getLanguageId() !== 'python') return;
    this.didOpen(model);
    model.onDidChangeContent(() => {
      if (!this.open.has(model.uri.toString())) return;
      this.notify('textDocument/didChange', {
        textDocument: { uri: model.uri.toString(), version: model.getVersionId() },
        contentChanges: [{ text: model.getValue() }],
      });
    });
    model.onWillDispose(() => {
      const uri = model.uri.toString();
      if (!this.open.delete(uri)) return;
      this.notify('textDocument/didClose', { textDocument: { uri } });
    });
  }

  didOpen(model) {
    const uri = model.uri.toString();
    if (!this.ready || model.isDisposed() || model.getLanguageId() !== 'python' || this.open.has(uri) || this.isLibrary(uri)) return;
    this.open.add(uri);
    this.notify('textDocument/didOpen', {
      textDocument: { uri, languageId: 'python', version: model.getVersionId(), text: model.getValue() },
    });
  }

  didSave(model) {
    const uri = model.uri.toString();
    if (this.open.has(uri)) this.notify('textDocument/didSave', { textDocument: { uri } });
  }

  diagnostics({ uri, diagnostics }) {
    const monaco = this.monaco;
    const model = monaco.editor.getModel(monaco.Uri.parse(uri));
    if (!model) return;
    const severity = { 1: monaco.MarkerSeverity.Error, 2: monaco.MarkerSeverity.Warning, 3: monaco.MarkerSeverity.Info, 4: monaco.MarkerSeverity.Hint };
    monaco.editor.setModelMarkers(
      model,
      'pyright',
      diagnostics.map((d) => ({
        startLineNumber: d.range.start.line + 1,
        startColumn: d.range.start.character + 1,
        endLineNumber: d.range.end.line + 1,
        endColumn: d.range.end.character + 1,
        message: d.message,
        severity: severity[d.severity] ?? monaco.MarkerSeverity.Error,
        source: 'Pyright',
        code: typeof d.code === 'object' ? String(d.code.value) : d.code !== undefined ? String(d.code) : undefined,
        tags: (d.tags || []).map((t) => (t === 1 ? monaco.MarkerTag.Unnecessary : monaco.MarkerTag.Deprecated)),
      })),
    );
  }

  // ---------- funkcie editora ----------
  pos(model, position) {
    return { textDocument: { uri: model.uri.toString() }, position: { line: position.lineNumber - 1, character: position.column - 1 } };
  }

  registerProviders() {
    const monaco = this.monaco;
    const self = this;

    monaco.languages.registerCompletionItemProvider('python', {
      triggerCharacters: ['.', '[', '"', "'"],
      async provideCompletionItems(model, position, context) {
        if (!self.ready || !self.open.has(model.uri.toString())) return { suggestions: [] };
        let result;
        try {
          result = await self.request('textDocument/completion', {
            ...self.pos(model, position),
            context: { triggerKind: context.triggerKind + 1, triggerCharacter: context.triggerCharacter },
          });
        } catch {
          return { suggestions: [] };
        }
        const items = Array.isArray(result) ? result : result?.items || [];
        const word = model.getWordUntilPosition(position);
        const defaultRange = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, position.column);
        return {
          incomplete: !Array.isArray(result) && !!result?.isIncomplete,
          suggestions: items.map((item) => {
            let range = defaultRange;
            let insertText = item.insertText ?? item.label;
            if (item.textEdit) {
              insertText = item.textEdit.newText;
              range = item.textEdit.range
                ? toRange(monaco, item.textEdit.range)
                : { insert: toRange(monaco, item.textEdit.insert), replace: toRange(monaco, item.textEdit.replace) };
            }
            return {
              label: item.labelDetails
                ? { label: item.label, detail: item.labelDetails.detail, description: item.labelDetails.description }
                : item.label,
              kind: completionKind(monaco, item.kind),
              detail: item.detail,
              documentation: markup(item.documentation),
              insertText,
              insertTextRules: item.insertTextFormat === 2 ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : 0,
              range,
              sortText: item.sortText,
              filterText: item.filterText,
              preselect: item.preselect,
              commitCharacters: item.commitCharacters,
              tags: item.tags?.includes(1) || item.deprecated ? [monaco.languages.CompletionItemTag.Deprecated] : [],
              additionalTextEdits: (item.additionalTextEdits || []).map((e) => ({ range: toRange(monaco, e.range), text: e.newText })),
              _lsp: item,
            };
          }),
        };
      },
      async resolveCompletionItem(item) {
        if (!self.ready || !item._lsp) return item;
        try {
          const r = await self.request('completionItem/resolve', item._lsp);
          if (r) {
            item.detail = r.detail ?? item.detail;
            item.documentation = markup(r.documentation) ?? item.documentation;
            if (r.additionalTextEdits) {
              // Automatický import (napr. „from math import sqrt“) – pridá sa po výbere návrhu.
              item.additionalTextEdits = r.additionalTextEdits.map((e) => ({ range: toRange(monaco, e.range), text: e.newText }));
            }
          }
        } catch {}
        return item;
      },
    });

    monaco.languages.registerHoverProvider('python', {
      async provideHover(model, position) {
        if (!self.ready || !self.open.has(model.uri.toString())) return null;
        try {
          const r = await self.request('textDocument/hover', self.pos(model, position));
          if (!r || !r.contents) return null;
          const contents = (Array.isArray(r.contents) ? r.contents : [r.contents]).map(markup).filter(Boolean);
          return { contents, range: r.range ? toRange(monaco, r.range) : undefined };
        } catch {
          return null;
        }
      },
    });

    monaco.languages.registerSignatureHelpProvider('python', {
      signatureHelpTriggerCharacters: ['(', ','],
      signatureHelpRetriggerCharacters: [')'],
      async provideSignatureHelp(model, position, _token, context) {
        if (!self.ready || !self.open.has(model.uri.toString())) return null;
        try {
          const r = await self.request('textDocument/signatureHelp', {
            ...self.pos(model, position),
            context: { triggerKind: context.triggerKind, triggerCharacter: context.triggerCharacter, isRetrigger: context.isRetrigger },
          });
          if (!r || !r.signatures?.length) return null;
          return {
            value: {
              activeSignature: r.activeSignature ?? 0,
              activeParameter: r.activeParameter ?? r.signatures[r.activeSignature ?? 0]?.activeParameter ?? 0,
              signatures: r.signatures.map((s) => ({
                label: s.label,
                documentation: markup(s.documentation),
                activeParameter: s.activeParameter,
                parameters: (s.parameters || []).map((p) => ({ label: p.label, documentation: markup(p.documentation) })),
              })),
            },
            dispose() {},
          };
        } catch {
          return null;
        }
      },
    });

    monaco.languages.registerDefinitionProvider('python', {
      async provideDefinition(model, position) {
        if (!self.ready || !self.open.has(model.uri.toString())) return null;
        let r;
        try {
          r = await self.request('textDocument/definition', self.pos(model, position));
        } catch {
          return null;
        }
        if (!r) return null;
        const list = Array.isArray(r) ? r : [r];
        const out = [];
        for (const loc of list) {
          const uri = monaco.Uri.parse(loc.targetUri || loc.uri);
          const range = loc.targetSelectionRange || loc.range;
          // Monaco potrebuje model cieľového súboru – načítame ho, ak ešte nie je otvorený.
          if (!monaco.editor.getModel(uri)) {
            try {
              const text = await flux.readAny(uri.fsPath);
              monaco.editor.createModel(text, 'python', uri);
            } catch {
              continue;
            }
          }
          out.push({ uri, range: toRange(monaco, range) });
        }
        return out;
      },
    });
  }
}
