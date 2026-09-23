// Šablóny nových súborov a projektov + krátke úryvky kódu (snippety) pre Python.
// V šablóne „$0“ označuje, kde bude kurzor po otvorení.

const HTML_PAGE = `<!DOCTYPE html>
<html lang="sk">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{{title}}</title>
</head>
<body>
  $0
</body>
</html>
`;

const HTML_WITH_ASSETS = `<!DOCTYPE html>
<html lang="sk">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{{title}}</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <main>
    <h1>{{title}}</h1>
    <p>Uprav tento text v index.html.</p>
    <button id="tlacidlo">Klikni</button>
  </main>

  <script src="script.js"></script>
</body>
</html>
`;

const CSS_BASE = `* {
  box-sizing: border-box;
}

body {
  margin: 0;
  min-height: 100vh;
  display: grid;
  place-items: center;
  font-family: system-ui, sans-serif;
  background: #f4f4f8;
  color: #1d1d24;
}

main {
  text-align: center;
}

button {
  padding: 10px 18px;
  border: 0;
  border-radius: 10px;
  background: #6b5cff;
  color: white;
  font-size: 16px;
  cursor: pointer;
}
`;

const JS_BASE = `const tlacidlo = document.querySelector('#tlacidlo');
let pocet = 0;

tlacidlo.addEventListener('click', () => {
  pocet++;
  tlacidlo.textContent = \`Kliknuté \${pocet}×\`;
  console.log('klik', pocet);
});
`;

export const TEMPLATES = [
  { id: 'empty', label: 'Prázdny súbor', detail: 'zadáš názov aj s príponou', name: '', files: null },
  { id: 'py', label: 'Python skript', detail: '.py', name: 'main.py', files: [{ name: '{{name}}', content: '$0\n' }] },
  {
    id: 'py-main',
    label: 'Python program (main)',
    detail: 'funkcia main() + spustenie',
    name: 'main.py',
    files: [{ name: '{{name}}', content: 'def main():\n    $0\n\n\nif __name__ == "__main__":\n    main()\n' }],
  },
  {
    id: 'py-input',
    label: 'Python – otázka a odpoveď (input)',
    detail: 'input() + print()',
    name: 'main.py',
    files: [{ name: '{{name}}', content: 'meno = input("Ako sa voláš? ")\nprint(f"Ahoj, {meno}!")\n$0' }],
  },
  {
    id: 'py-tkinter',
    label: 'Python – okno (tkinter)',
    detail: 'aplikácia s tlačidlom',
    name: 'okno.py',
    files: [
      {
        name: '{{name}}',
        content:
          'import tkinter as tk\n\n\ndef klik():\n    popis.config(text="Kliknuté!")\n\n\nokno = tk.Tk()\nokno.title("Moja aplikácia")\nokno.geometry("360x200")\n\npopis = tk.Label(okno, text="Ahoj!", font=("Segoe UI", 16))\npopis.pack(pady=24)\n\ntk.Button(okno, text="Klikni", command=klik).pack()\n$0\nokno.mainloop()\n',
      },
    ],
  },
  {
    id: 'py-turtle',
    label: 'Python – kreslenie (turtle)',
    detail: 'korytnačka kreslí hviezdu',
    name: 'kreslenie.py',
    files: [
      {
        name: '{{name}}',
        content:
          'import turtle\n\nt = turtle.Turtle()\nt.speed(0)\nt.color("purple")\n\nfor _ in range(36):\n    t.forward(200)\n    t.left(170)\n$0\nturtle.done()\n',
      },
    ],
  },
  {
    id: 'py-pygame',
    label: 'Python – hra (pygame)',
    detail: 'okno s pohybujúcim sa štvorcom',
    name: 'hra.py',
    files: [
      {
        name: '{{name}}',
        content:
          'import pygame\n\npygame.init()\nobrazovka = pygame.display.set_mode((640, 480))\npygame.display.set_caption("Moja hra")\nhodiny = pygame.time.Clock()\n\nx, y = 300, 220\nbezi = True\nwhile bezi:\n    for udalost in pygame.event.get():\n        if udalost.type == pygame.QUIT:\n            bezi = False\n\n    klavesy = pygame.key.get_pressed()\n    if klavesy[pygame.K_LEFT]:\n        x -= 5\n    if klavesy[pygame.K_RIGHT]:\n        x += 5\n    if klavesy[pygame.K_UP]:\n        y -= 5\n    if klavesy[pygame.K_DOWN]:\n        y += 5\n\n    obrazovka.fill((30, 30, 40))\n    pygame.draw.rect(obrazovka, (139, 123, 255), (x, y, 40, 40))\n    $0\n    pygame.display.flip()\n    hodiny.tick(60)\n\npygame.quit()\n',
      },
    ],
  },
  { id: 'html', label: 'HTML stránka', detail: 'základná kostra (<!DOCTYPE>, head, body)', name: 'index.html', files: [{ name: '{{name}}', content: HTML_PAGE }] },
  {
    id: 'web',
    label: 'Webový projekt (HTML + CSS + JS)',
    detail: 'priečinok s index.html, style.css, script.js',
    name: 'moj-web',
    project: true,
    files: [
      { name: 'index.html', content: HTML_WITH_ASSETS, open: true },
      { name: 'style.css', content: CSS_BASE },
      { name: 'script.js', content: JS_BASE },
    ],
  },
  { id: 'css', label: 'CSS štýly', detail: '.css', name: 'style.css', files: [{ name: '{{name}}', content: CSS_BASE.replace('main {', '$0main {') }] },
  { id: 'js', label: 'JavaScript', detail: '.js (spustí sa cez Node.js)', name: 'script.js', files: [{ name: '{{name}}', content: "console.log('Ahoj!');\n$0" }] },
];

// Úryvky pre Python – napíš skratku a stlač Tab / Enter.
export const PY_SNIPPETS = [
  { label: 'main', detail: 'if __name__ == "__main__":', body: 'def main():\n\t${1:pass}\n\n\nif __name__ == "__main__":\n\tmain()\n' },
  { label: 'ifmain', detail: 'if __name__ == "__main__":', body: 'if __name__ == "__main__":\n\t${1:main()}' },
  { label: 'def', detail: 'nová funkcia', body: 'def ${1:nazov}(${2}):\n\t${3:pass}' },
  { label: 'class', detail: 'nová trieda', body: 'class ${1:Nazov}:\n\tdef __init__(self${2}):\n\t\t${3:pass}' },
  { label: 'for', detail: 'for … in range', body: 'for ${1:i} in range(${2:10}):\n\t${3:print($1)}' },
  { label: 'fore', detail: 'for … in zoznam', body: 'for ${1:prvok} in ${2:zoznam}:\n\t${3:print($1)}' },
  { label: 'while', detail: 'cyklus while', body: 'while ${1:True}:\n\t${2:pass}' },
  { label: 'if', detail: 'podmienka', body: 'if ${1:podmienka}:\n\t${2:pass}' },
  { label: 'ifelse', detail: 'if / else', body: 'if ${1:podmienka}:\n\t${2:pass}\nelse:\n\t${3:pass}' },
  { label: 'try', detail: 'try / except', body: 'try:\n\t${1:pass}\nexcept ${2:Exception} as e:\n\t${3:print(e)}' },
  { label: 'with', detail: 'otvoriť súbor', body: 'with open("${1:subor.txt}", encoding="utf-8") as f:\n\t${2:obsah = f.read()}' },
  { label: 'input', detail: 'otázka pre používateľa', body: '${1:odpoved} = input("${2:Otázka? }")' },
  { label: 'inputint', detail: 'načítať číslo', body: '${1:cislo} = int(input("${2:Zadaj číslo: }"))' },
  { label: 'printf', detail: 'print s f-reťazcom', body: 'print(f"${1:{premenna}}")' },
  { label: 'lambda', detail: 'anonymná funkcia', body: 'lambda ${1:x}: ${2:x}' },
  { label: 'listcomp', detail: '[… for … in …]', body: '[${1:x} for ${2:x} in ${3:zoznam}]' },
];
