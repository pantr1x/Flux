# Flux themes

A Flux theme is **one small text file**: one line per value, a name and a color. You only change the values.

```conf
# My theme – Flux theme
name        My theme
mode        dark        # dark or light

# code colors
fg          #e2e2ea     # normal text
comment     #7a7a90
keyword     #c792ea     # if, for, return, def…
string      #a8db8a

# app colors (optional)
background  #16161a     # around the panels
panel       #1e1e24     # editor and sidebar
accent      #8b7bff     # buttons and highlights
```

- A line is `key` + spaces + value. `key = value` and `key: value` work too.
- Lines starting with `#` are comments. A comment after a value needs a space before and after the `#` (`#ff0000 # red`).
- Colors are `#rrggbb` (or short `#rgb`).
- Every key is optional. Code colors you leave out come from `based_on` (another theme id, e.g. `based_on dracula`) or from the default dark/light theme. App colors you leave out keep Flux's own colors.

## Make your own theme in Flux

1. Settings → *Environment* → *Theme and window* → **New theme from this one**.
2. Flux writes `my-theme.conf` with every color of the current theme and opens it.
3. Change a value and **save** – the theme updates right away.

Your themes live in the **themes folder** (button *Themes folder*): `%APPDATA%\Flux\themes` on Windows, `~/.config/Flux/themes` on Linux (the Flux settings folder + `themes`). A file there with the same name as a built-in theme replaces it.

## Share a theme with everyone

Built-in themes are the files in [`themes/`](../themes) of this repository. To add one:

1. Fork the repository.
2. Add `themes/<your-theme>.conf` (lowercase, words joined with `-`, e.g. `tokyo-storm.conf`).
3. Open a pull request. Flux picks up every `.conf` file in `themes/` by itself when it is built – no other file has to change.

## All keys

| Key | What it colors |
|---|---|
| `name` | name shown in Settings |
| `mode` | `dark` or `light` |
| `based_on` | theme id to take missing colors from (`vscode-dark`, `dracula`, `nord`, `gruvbox-dark`…) |
| **Code** | |
| `fg` (or `foreground`) | normal text |
| `comment` | comments |
| `keyword` | `if`, `for`, `return`, `def`, `class`… |
| `storage` (or `builtin`) | `true`, `None`, `self`, `int`… |
| `string` | text in quotes |
| `number` | numbers |
| `type` (or `class`) | class and type names |
| `function` | function names |
| `variable` | variables |
| `parameter` | function parameters |
| `property` | `object.property` |
| `constant` | `CONSTANTS` |
| `tag` | HTML tags (`<div>`) |
| `attr` (or `attribute`) | HTML attributes (`class="…"`) |
| `delimiter` (or `punctuation`, `operator`) | brackets, commas, operators |
| `regexp` (or `regex`) | regular expressions |
| **App** (optional) | |
| `background` | the window around the panels |
| `panel` | the editor and the sidebar |
| `text` | menus, file names, buttons |
| `text2` | hints and small text |
| `border` (or `line`) | lines between parts |
| `accent` | buttons and highlights (used when you haven't picked an accent color yourself) |

Your own colors in Settings → *Fonts and colors* → *App colors* still win over the theme's app colors.
