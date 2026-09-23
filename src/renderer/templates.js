// Templates for new files and projects + short Python snippets.
// „$0“ marks where the cursor goes after the file opens. Labels are translated with t().

export const HTML_PAGE = `<!DOCTYPE html>
<html lang="en">
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
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{{title}}</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <main>
    <h1>{{title}}</h1>
    <p>Edit this text in index.html.</p>
    <button id="button">Click me</button>
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

const JS_BASE = `const button = document.querySelector('#button');
let count = 0;

button.addEventListener('click', () => {
  count++;
  button.textContent = \`Clicked \${count}×\`;
  console.log('click', count);
});
`;

export const TEMPLATES = [
  { id: 'empty', label: 'Empty file', detail: 'type the name with an extension', name: '', files: null },
  { id: 'py', label: 'Python script', detail: '.py', name: 'main.py', files: [{ name: '{{name}}', content: '$0\n' }] },
  {
    id: 'py-main',
    label: 'Python program (main)',
    detail: 'main() function + entry point',
    name: 'main.py',
    files: [{ name: '{{name}}', content: 'def main():\n    $0\n\n\nif __name__ == "__main__":\n    main()\n' }],
  },
  {
    id: 'py-input',
    label: 'Python – question and answer (input)',
    detail: 'input() + print()',
    name: 'main.py',
    files: [{ name: '{{name}}', content: 'name = input("What is your name? ")\nprint(f"Hello, {name}!")\n$0' }],
  },
  {
    id: 'py-tkinter',
    label: 'Python – window (tkinter)',
    detail: 'app with a button',
    name: 'window.py',
    files: [
      {
        name: '{{name}}',
        content:
          'import tkinter as tk\n\n\ndef on_click():\n    label.config(text="Clicked!")\n\n\nwindow = tk.Tk()\nwindow.title("My app")\nwindow.geometry("360x200")\n\nlabel = tk.Label(window, text="Hello!", font=("Segoe UI", 16))\nlabel.pack(pady=24)\n\ntk.Button(window, text="Click me", command=on_click).pack()\n$0\nwindow.mainloop()\n',
      },
    ],
  },
  {
    id: 'py-turtle',
    label: 'Python – drawing (turtle)',
    detail: 'the turtle draws a star',
    name: 'drawing.py',
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
    label: 'Python – game (pygame)',
    detail: 'window with a moving square',
    name: 'game.py',
    files: [
      {
        name: '{{name}}',
        content:
          'import pygame\n\npygame.init()\nscreen = pygame.display.set_mode((640, 480))\npygame.display.set_caption("My game")\nclock = pygame.time.Clock()\n\nx, y = 300, 220\nrunning = True\nwhile running:\n    for event in pygame.event.get():\n        if event.type == pygame.QUIT:\n            running = False\n\n    keys = pygame.key.get_pressed()\n    if keys[pygame.K_LEFT]:\n        x -= 5\n    if keys[pygame.K_RIGHT]:\n        x += 5\n    if keys[pygame.K_UP]:\n        y -= 5\n    if keys[pygame.K_DOWN]:\n        y += 5\n\n    screen.fill((30, 30, 40))\n    pygame.draw.rect(screen, (139, 123, 255), (x, y, 40, 40))\n    $0\n    pygame.display.flip()\n    clock.tick(60)\n\npygame.quit()\n',
      },
    ],
  },
  { id: 'html', label: 'HTML page', detail: 'basic skeleton (<!DOCTYPE>, head, body)', name: 'index.html', files: [{ name: '{{name}}', content: HTML_PAGE }] },
  {
    id: 'web',
    label: 'Web project (HTML + CSS + JS)',
    detail: 'folder with index.html, style.css, script.js',
    name: 'my-website',
    project: true,
    files: [
      { name: 'index.html', content: HTML_WITH_ASSETS, open: true },
      { name: 'style.css', content: CSS_BASE },
      { name: 'script.js', content: JS_BASE },
    ],
  },
  { id: 'css', label: 'CSS styles', detail: '.css', name: 'style.css', files: [{ name: '{{name}}', content: CSS_BASE.replace('main {', '$0main {') }] },
  { id: 'js', label: 'JavaScript', detail: '.js (runs with Node.js)', name: 'script.js', files: [{ name: '{{name}}', content: "console.log('Hello!');\n$0" }] },
];

// Python snippets – type the shortcut and press Tab / Enter.
export const PY_SNIPPETS = [
  { label: 'main', detail: 'if __name__ == "__main__":', body: 'def main():\n\t${1:pass}\n\n\nif __name__ == "__main__":\n\tmain()\n' },
  { label: 'ifmain', detail: 'if __name__ == "__main__":', body: 'if __name__ == "__main__":\n\t${1:main()}' },
  { label: 'def', detail: 'new function', body: 'def ${1:name}(${2}):\n\t${3:pass}' },
  { label: 'class', detail: 'new class', body: 'class ${1:Name}:\n\tdef __init__(self${2}):\n\t\t${3:pass}' },
  { label: 'for', detail: 'for … in range', body: 'for ${1:i} in range(${2:10}):\n\t${3:print($1)}' },
  { label: 'fore', detail: 'for … in list', body: 'for ${1:item} in ${2:items}:\n\t${3:print($1)}' },
  { label: 'while', detail: 'while loop', body: 'while ${1:True}:\n\t${2:pass}' },
  { label: 'if', detail: 'condition', body: 'if ${1:condition}:\n\t${2:pass}' },
  { label: 'ifelse', detail: 'if / else', body: 'if ${1:condition}:\n\t${2:pass}\nelse:\n\t${3:pass}' },
  { label: 'try', detail: 'try / except', body: 'try:\n\t${1:pass}\nexcept ${2:Exception} as e:\n\t${3:print(e)}' },
  { label: 'with', detail: 'open a file', body: 'with open("${1:file.txt}", encoding="utf-8") as f:\n\t${2:text = f.read()}' },
  { label: 'input', detail: 'ask the user', body: '${1:answer} = input("${2:Question? }")' },
  { label: 'inputint', detail: 'read a number', body: '${1:number} = int(input("${2:Enter a number: }"))' },
  { label: 'printf', detail: 'print with an f-string', body: 'print(f"${1:{value}}")' },
  { label: 'lambda', detail: 'anonymous function', body: 'lambda ${1:x}: ${2:x}' },
  { label: 'listcomp', detail: '[… for … in …]', body: '[${1:x} for ${2:x} in ${3:items}]' },
];
