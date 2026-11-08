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

## Theme store

Settings → *Environment* → *Theme and window* → **Theme store** lists themes made by other people. Each card shows a small piece of code in the theme's own colors; **Install** downloads the file into your themes folder and switches to it right away, **Use** switches back to it later, and the trash button removes it again (you can install it again any time). Your own themes are never removed from there.

The store is the folder [`themes/store/`](../themes/store) of this repository: one `.conf` file per theme plus [`index.json`](../themes/store/index.json) with the name, author and one sentence about each.

## Share your theme

Anyone can add a theme to the store:

1. Make your theme in Flux (see above) and check it in both the editor and the menus.
2. Fork the repository on GitHub.
3. Copy your file to `themes/store/<your-theme>.conf` – lowercase, words joined with `-`, e.g. `tokyo-storm.conf`. Add a line `author  Your Name`.
4. Add an entry to `themes/store/index.json`:

   ```json
   { "id": "tokyo-storm", "name": "Tokyo Storm", "author": "Your Name", "mode": "dark", "description": "One short sentence about the look." }
   ```

   `id` must be the file name without `.conf`.
5. Open a pull request. A test (`cargo test -p flux-native themes_store`) checks that every file is listed in `index.json`, can be read, and has an author and a description.

Once it is merged, your theme shows up in every Flux's store – no new Flux version is needed.

**Built-in themes** are the files directly in [`themes/`](../themes) (not in `store/`): Flux bundles every `.conf` file there when it is built. Only add one there when it should come with Flux for everyone.

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
