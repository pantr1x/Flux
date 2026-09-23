// Lorem Ipsum – výplňový text do webových stránok.
const WORDS = 'lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua ut enim ad minim veniam quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur excepteur sint occaecat cupidatat non proident sunt in culpa qui officia deserunt mollit anim id est laborum'.split(' ');

function sentence(n = 8 + Math.floor(Math.random() * 8)) {
  const w = Array.from({ length: n }, () => WORDS[Math.floor(Math.random() * WORDS.length)]);
  w[0] = w[0][0].toUpperCase() + w[0].slice(1);
  return `${w.join(' ')}.`;
}

function paragraph(sentences = 4) {
  return ['Lorem ipsum dolor sit amet, consectetur adipiscing elit.', ...Array.from({ length: sentences - 1 }, () => sentence())].join(' ');
}

export function activate(flux) {
  const para = paragraph();
  for (const lang of ['html', 'markdown', 'plaintext', 'css', 'javascript', 'python']) {
    flux.snippets.add(lang, 'lorem', para, 'Lorem ipsum paragraph');
  }
  flux.snippets.add('html', 'lorem-p', `<p>${paragraph()}</p>`, '<p> with lorem ipsum');
  flux.snippets.add('html', 'lorem-list', `<ul>\n\t<li>${sentence(4)}</li>\n\t<li>${sentence(5)}</li>\n\t<li>${sentence(3)}</li>\n</ul>`, 'List with lorem ipsum');

  flux.commands.register('paragraph', 'Insert a paragraph', () => flux.insertText(paragraph()), { key: 'Ctrl+Alt+L' });
  flux.commands.register('three', 'Insert 3 paragraphs', () => flux.insertText([paragraph(), paragraph(5), paragraph(3)].join('\n\n')));
  flux.commands.register('sentence', 'Insert a sentence', () => flux.insertText(sentence()));
}
