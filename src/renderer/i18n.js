// Preklady: rozhranie je v angličtine, iné jazyky sa sťahujú z GitHubu (locales/<kód>.json)
// a ukladajú sa do počítača – nie sú súčasťou inštalátora.
let dict = {};

export function setLocale(d) {
  dict = d || {};
}

// t('Hello {name}', { name }) → preložený text s dosadenými hodnotami.
export function t(text, vars) {
  let out = dict[text] ?? text;
  if (vars) out = out.replace(/\{(\w+)\}/g, (_, k) => (vars[k] ?? `{${k}}`));
  return out;
}

// Preloží statické texty v HTML: elementy s data-t a všetky title="…".
export function translateDom(root = document) {
  for (const el of root.querySelectorAll('[data-t]')) {
    el.dataset.tSrc ??= el.textContent;
    el.textContent = t(el.dataset.tSrc);
  }
  for (const el of root.querySelectorAll('[title]')) {
    el.dataset.titleSrc ??= el.title;
    el.title = t(el.dataset.titleSrc);
  }
}
