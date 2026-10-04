// Údaje pre návrhy (ako VS Code): značky a atribúty HTML, vlastnosti CSS, členy objektov, moduly.
// Zapísané ako text, rozložia sa raz pri prvom použití (lacné na pamäť aj na čítanie).

// značka|popis
pub const TAGS: &str = "\
a|Hyperlink to another page, file or place
abbr|Abbreviation or acronym
address|Contact information
area|Clickable area inside an image map
article|Self-contained composition (post, card, story)
aside|Content aside from the main content
audio|Sound or music player
b|Bold text without extra importance
base|Base URL for all relative links
bdi|Text isolated from the surrounding direction
bdo|Overrides the text direction
big|Bigger text (deprecated – use CSS)
blockquote|Quotation from another source
body|The visible content of the page
br|Line break
button|Clickable button
canvas|Drawing surface for scripts (2D, WebGL)
caption|Title of a table
center|Centered content (deprecated – use CSS)
cite|Title of a creative work
code|Fragment of computer code
col|Column inside a colgroup
colgroup|Group of table columns
data|Value with a machine-readable form
datalist|Suggestions for an input
dd|Description in a description list
del|Deleted text
details|Disclosure widget that opens and closes
dfn|Term being defined
dialog|Dialog box or popup window
div|Generic block container
dl|Description list
dt|Term in a description list
em|Emphasized text
embed|Embedded external content
fieldset|Group of form controls
figcaption|Caption of a figure
figure|Self-contained figure (image, diagram, code)
font|Font face, size and color (deprecated – use CSS)
footer|Footer of a page or section
form|Form that sends user input
h1|Heading, level 1 (most important)
h2|Heading, level 2
h3|Heading, level 3
h4|Heading, level 4
h5|Heading, level 5
h6|Heading, level 6
head|Information about the page (title, styles, scripts)
header|Header of a page or section
hgroup|Heading with subheadings
hr|Thematic break (horizontal line)
html|Root of the page
i|Text in an alternate voice (often italic)
iframe|Another page embedded in this one
img|Image
input|Input field (text, checkbox, number, …)
ins|Inserted text
kbd|Keyboard input
label|Caption for a form control
legend|Caption of a fieldset
li|List item
link|Link to a stylesheet, icon or other resource
main|Main content of the page
map|Image map
mark|Highlighted text
marquee|Scrolling text (deprecated)
menu|List of commands
meta|Page metadata (charset, viewport, description)
meter|Value within a known range
nav|Navigation links
noscript|Content shown when scripts are off
object|External resource (plugin, PDF, …)
ol|Ordered (numbered) list
optgroup|Group of options in a select
option|Option in a select or datalist
output|Result of a calculation
p|Paragraph
param|Parameter for an object
picture|Image with several sources
pre|Preformatted text (keeps spaces)
progress|Progress bar
q|Short inline quotation
rp|Fallback parentheses for ruby
rt|Ruby text (pronunciation)
ruby|Ruby annotation (East Asian text)
s|Text that is no longer correct
samp|Sample output of a program
script|JavaScript code or file
search|Search section
section|Section of a document
select|Drop-down list
slot|Placeholder in a web component
small|Side comments, small print
source|Media source for picture, audio or video
span|Generic inline container
strike|Strikethrough text (deprecated – use s or CSS)
strong|Important text (bold)
style|CSS styles
sub|Subscript
summary|Visible heading of a details element
sup|Superscript
svg|Scalable vector graphic
table|Table
tbody|Body rows of a table
td|Table cell
template|HTML that is not shown, for scripts
textarea|Multi-line text field
tfoot|Footer rows of a table
th|Table header cell
thead|Header rows of a table
time|Date or time
title|Title of the page (browser tab)
tr|Table row
track|Subtitles for audio or video
tt|Teletype text (deprecated – use code)
u|Underlined text
ul|Unordered (bulleted) list
var|Variable in math or code
video|Video player
wbr|Possible line break
";

// atribúty pre všetky značky
pub const GLOBAL_ATTRS: &str = "class id style title lang dir hidden tabindex contenteditable draggable spellcheck translate accesskey role aria-label aria-hidden aria-describedby data-";

pub const EVENT_ATTRS: &str = "onclick ondblclick onchange oninput onsubmit onkeydown onkeyup onkeypress onfocus onblur onload onerror onmouseover onmouseout onmouseenter onmouseleave onmousedown onmouseup onscroll onresize oncontextmenu ondrag ondrop";

// značka: jej vlastné atribúty
pub const TAG_ATTRS: &str = "\
a: href target rel download hreflang type referrerpolicy ping
area: alt coords shape href target download rel
audio: src controls autoplay loop muted preload crossorigin
base: href target
blockquote: cite
body: onload onunload
button: type name value disabled form formaction formmethod autofocus popovertarget
canvas: width height
col: span
colgroup: span
data: value
del: cite datetime
details: open name
dialog: open
embed: src type width height
fieldset: disabled form name
font: color face size
form: action method enctype target autocomplete novalidate name accept-charset
html: lang xmlns
iframe: src srcdoc name width height allow allowfullscreen loading referrerpolicy sandbox title
img: src alt width height loading decoding srcset sizes usemap crossorigin referrerpolicy fetchpriority
input: type name value placeholder required disabled readonly checked autofocus autocomplete min max step minlength maxlength pattern size multiple accept list form
ins: cite datetime
label: for form
li: value
link: rel href type media sizes crossorigin integrity as hreflang
map: name
marquee: behavior direction scrollamount loop
meta: name content charset http-equiv property
meter: value min max low high optimum
object: data type name width height form
ol: reversed start type
optgroup: label disabled
option: value selected disabled label
output: for form name
progress: value max
q: cite
script: src type defer async crossorigin integrity nomodule referrerpolicy
select: name multiple required disabled size autofocus form
slot: name
source: src srcset type media sizes width height
style: media
td: colspan rowspan headers
template: shadowrootmode
textarea: name rows cols placeholder required disabled readonly maxlength minlength wrap autofocus
th: colspan rowspan scope abbr headers
time: datetime
track: src kind srclang label default
video: src controls autoplay loop muted poster width height preload playsinline crossorigin
";

// atribúty bez hodnoty (vloží sa len meno)
pub const BOOL_ATTRS: &str = "hidden disabled checked required readonly autofocus multiple selected controls autoplay loop muted defer async novalidate open reversed playsinline allowfullscreen default nomodule ismap formnovalidate download";

// hodnoty atribútov: „atribút:“ pre všetky značky, „značka.atribút:“ len pre tú značku; s „|“ sa delí podľa „|“, inak medzerami
pub const ATTR_VALUES: &str = "\
input.type: text password email number tel url search date time datetime-local month week color range checkbox radio file submit reset button hidden image
button.type: button submit reset
script.type: module text/javascript importmap application/json
ol.type: 1 a A i I
link.rel: stylesheet icon preconnect preload prefetch manifest canonical alternate apple-touch-icon modulepreload
a.rel: noopener noreferrer nofollow external help license next prev
target: _blank _self _parent _top
method: get post dialog
enctype: application/x-www-form-urlencoded multipart/form-data text/plain
loading: lazy eager
decoding: async sync auto
preload: none metadata auto
crossorigin: anonymous use-credentials
referrerpolicy: no-referrer no-referrer-when-downgrade origin origin-when-cross-origin same-origin strict-origin strict-origin-when-cross-origin unsafe-url
dir: ltr rtl auto
lang: en sk cs de fr es it pl pt uk hu ru
autocomplete: on off name email username current-password new-password tel street-address postal-code country
wrap: soft hard
contenteditable: true false plaintext-only
draggable: true false
spellcheck: true false
translate: yes no
meta.name: viewport description keywords author theme-color robots color-scheme generator
meta.charset: UTF-8
meta.http-equiv: refresh content-type x-ua-compatible content-security-policy
meta.content: width=device-width, initial-scale=1.0|
track.kind: subtitles captions descriptions chapters metadata
th.scope: row col rowgroup colgroup
sandbox: allow-scripts allow-same-origin allow-forms allow-popups allow-modals allow-downloads
font.face: Arial|Verdana|Tahoma|Georgia|Times New Roman|Courier New|sans-serif|serif|monospace
font.size: 1 2 3 4 5 6 7
font.color: red blue green black white gray orange purple
role: button link navigation main banner dialog alert tab tablist tabpanel list listitem img presentation
fetchpriority: high low auto
shape: rect circle poly default
template.shadowrootmode: open closed
marquee.behavior: scroll slide alternate
marquee.direction: left right up down
";

// vlastnosť CSS: hodnoty (rovnako: „|“ alebo medzery)
pub const CSS: &str = "\
align-content: center flex-start flex-end space-between space-around space-evenly stretch normal
align-items: center flex-start flex-end start end stretch baseline normal
align-self: auto center flex-start flex-end stretch baseline
animation: none
animation-delay:
animation-direction: normal reverse alternate alternate-reverse
animation-duration:
animation-fill-mode: none forwards backwards both
animation-iteration-count: infinite 1
animation-name: none
animation-play-state: running paused
animation-timing-function: ease linear ease-in ease-out ease-in-out step-start step-end cubic-bezier()
aspect-ratio: auto|1 / 1|16 / 9|4 / 3
backdrop-filter: none blur() brightness() saturate()
backface-visibility: visible hidden
background: none transparent linear-gradient() radial-gradient() url()
background-attachment: scroll fixed local
background-blend-mode: normal multiply screen overlay darken lighten
background-clip: border-box padding-box content-box text
background-color: transparent currentColor
background-image: none url() linear-gradient() radial-gradient() conic-gradient()
background-origin: border-box padding-box content-box
background-position: center top bottom left right
background-repeat: no-repeat repeat repeat-x repeat-y space round
background-size: cover contain auto
border: none|1px solid
border-bottom: none|1px solid
border-collapse: collapse separate
border-color: transparent currentColor
border-left: none|1px solid
border-radius: 50%
border-right: none|1px solid
border-spacing:
border-style: none solid dashed dotted double groove ridge inset outset hidden
border-top: none|1px solid
border-width: thin medium thick
bottom: auto 0
box-shadow: none inset
box-sizing: border-box content-box
caret-color: auto transparent
clear: none left right both
clip-path: none circle() ellipse() inset() polygon()
color: currentColor inherit transparent
column-count: auto
column-gap: normal
columns: auto
content: '' none normal attr() counter()
cursor: auto default pointer text move grab grabbing not-allowed wait progress help crosshair zoom-in zoom-out col-resize row-resize ew-resize ns-resize none
direction: ltr rtl
display: block inline inline-block flex inline-flex grid inline-grid none contents table list-item flow-root
fill: currentColor none
filter: none blur() brightness() contrast() grayscale() invert() saturate() sepia() drop-shadow() hue-rotate()
flex: 1|auto|none|0 0 auto
flex-basis: auto 0 content
flex-direction: row column row-reverse column-reverse
flex-flow: row wrap|column nowrap|row nowrap|column wrap
flex-grow: 0 1
flex-shrink: 0 1
flex-wrap: nowrap wrap wrap-reverse
float: none left right
font: inherit
font-family: system-ui|sans-serif|serif|monospace|cursive|Arial, sans-serif|Helvetica, sans-serif|Verdana, sans-serif|Georgia, serif|'Segoe UI', sans-serif|'Courier New', monospace
font-size: small medium large x-large xx-large smaller larger
font-style: normal italic oblique
font-variant: normal small-caps
font-weight: normal bold lighter bolder 100 200 300 400 500 600 700 800 900
gap: 0
grid-area: auto
grid-auto-flow: row column dense
grid-column: auto|span 2|1 / -1
grid-row: auto|span 2|1 / -1
grid-template-areas: none
grid-template-columns: none repeat() minmax() auto 1fr
grid-template-rows: none repeat() minmax() auto 1fr
height: auto 100% 100vh fit-content min-content max-content
inset: 0 auto
isolation: auto isolate
justify-content: center flex-start flex-end start end space-between space-around space-evenly stretch normal
justify-items: center start end stretch normal
justify-self: auto center start end stretch
left: auto 0
letter-spacing: normal
line-height: normal 1.5
list-style: none disc circle square decimal inside outside
list-style-type: none disc circle square decimal lower-alpha upper-alpha lower-roman upper-roman
margin: 0 auto
margin-bottom: 0 auto
margin-left: 0 auto
margin-right: 0 auto
margin-top: 0 auto
max-height: none 100%
max-width: none 100%
min-height: auto 0 100vh
min-width: auto 0
mix-blend-mode: normal multiply screen overlay darken lighten difference
object-fit: fill contain cover none scale-down
object-position: center top bottom left right
opacity: 0 0.5 1
order: 0
outline: none
outline-offset:
overflow: visible hidden scroll auto clip
overflow-wrap: normal break-word anywhere
overflow-x: visible hidden scroll auto clip
overflow-y: visible hidden scroll auto clip
padding: 0
padding-bottom: 0
padding-left: 0
padding-right: 0
padding-top: 0
place-content: center start end space-between
place-items: center start end stretch
pointer-events: auto none
position: static relative absolute fixed sticky
resize: none both horizontal vertical
right: auto 0
row-gap: normal
scroll-behavior: auto smooth
scroll-snap-type: none|x mandatory|y mandatory
stroke: currentColor none
table-layout: auto fixed
text-align: left center right justify start end
text-decoration: none underline overline line-through
text-indent:
text-overflow: clip ellipsis
text-shadow: none
text-transform: none uppercase lowercase capitalize
top: auto 0
transform: none translate() translateX() translateY() rotate() scale() skew() matrix()
transform-origin: center top left right bottom
transition: none all
transition-delay:
transition-duration:
transition-property: all none opacity transform color background-color
transition-timing-function: ease linear ease-in ease-out ease-in-out
user-select: auto none text all
vertical-align: baseline top middle bottom text-top text-bottom sub super
visibility: visible hidden collapse
white-space: normal nowrap pre pre-wrap pre-line break-spaces
width: auto 100% fit-content min-content max-content
will-change: auto transform opacity
word-break: normal break-all keep-all break-word
word-spacing: normal
z-index: auto 0 1 10 100
";

// hodnoty spoločné pre všetky vlastnosti a farby
pub const CSS_COMMON: &str = "inherit initial unset revert var() calc() min() max() clamp()";
pub const CSS_COLORS: &str = "black white red green blue yellow orange purple pink gray grey brown cyan magenta lime navy teal olive maroon silver gold transparent currentColor rgb() rgba() hsl() hsla() #";
pub const CSS_PSEUDO: &str = "hover active focus focus-visible focus-within visited checked disabled enabled first-child last-child nth-child() nth-of-type() first-of-type last-of-type not() is() where() has() root empty placeholder-shown required invalid valid target before after placeholder selection first-letter first-line marker";
pub const CSS_AT: &[(&str, &str)] = &[
    ("media", "media (${1:max-width: 600px}) {\n\t$0\n}"),
    ("keyframes", "keyframes ${1:name} {\n\tfrom { $2 }\n\tto { $0 }\n}"),
    ("import", "import url(\"$1\");$0"),
    ("font-face", "font-face {\n\tfont-family: \"$1\";\n\tsrc: url(\"$2\");$0\n}"),
    ("supports", "supports (${1:display: grid}) {\n\t$0\n}"),
    ("container", "container (${1:min-width: 400px}) {\n\t$0\n}"),
    ("layer", "layer ${1:name} {\n\t$0\n}"),
];

// členy objektov: „jazyk objekt: podpis;podpis;…“ (podpis so zátvorkou = metóda, bez = vlastnosť)
// objekt „*“ = keď objekt nepoznáme (bežné metódy reťazcov, zoznamov…)
pub const MEMBERS: &str = "\
js console: log(...data);error(...data);warn(...data);info(...data);debug(...data);table(data);clear();dir(obj);time(label);timeEnd(label);count(label);group(label);groupEnd();assert(condition, ...data);trace()
js document: getElementById(id);querySelector(selector);querySelectorAll(selector);createElement(tag);createTextNode(text);addEventListener(type, listener);removeEventListener(type, listener);getElementsByClassName(names);getElementsByTagName(tag);body;head;title;documentElement;cookie;activeElement;forms;images;links;readyState;URL;location
js window: alert(message);confirm(message);prompt(message, default);setTimeout(fn, ms);setInterval(fn, ms);clearTimeout(id);clearInterval(id);requestAnimationFrame(fn);addEventListener(type, listener);scrollTo(x, y);open(url);close();getComputedStyle(element);matchMedia(query);fetch(url, options);innerWidth;innerHeight;scrollX;scrollY;location;localStorage;sessionStorage;navigator;history;devicePixelRatio
js Math: random();floor(x);ceil(x);round(x);abs(x);max(...values);min(...values);pow(x, y);sqrt(x);cbrt(x);sin(x);cos(x);tan(x);atan2(y, x);log(x);log10(x);log2(x);exp(x);sign(x);trunc(x);hypot(...values);PI;E;SQRT2;LN2
js JSON: parse(text);stringify(value, replacer, space)
js Object: keys(obj);values(obj);entries(obj);assign(target, ...sources);freeze(obj);fromEntries(entries);create(proto);defineProperty(obj, key, desc);getPrototypeOf(obj);hasOwn(obj, key);groupBy(items, fn);is(a, b)
js Array: from(items);isArray(value);of(...items)
js Number: parseInt(text);parseFloat(text);isInteger(value);isNaN(value);isFinite(value);MAX_SAFE_INTEGER;MIN_SAFE_INTEGER;EPSILON
js String: fromCharCode(...codes);raw(strings)
js Promise: all(promises);allSettled(promises);race(promises);any(promises);resolve(value);reject(reason);withResolvers()
js Date: now();parse(text);UTC(year, month)
js localStorage: getItem(key);setItem(key, value);removeItem(key);clear();key(index);length
js sessionStorage: getItem(key);setItem(key, value);removeItem(key);clear();key(index);length
js navigator: clipboard;userAgent;language;languages;onLine;geolocation;mediaDevices;share(data);vibrate(pattern)
js navigator.clipboard: writeText(text);readText();write(items);read()
js location: href;pathname;search;hash;host;hostname;origin;protocol;port;reload();assign(url);replace(url)
js history: back();forward();go(delta);pushState(state, title, url);replaceState(state, title, url);length;state
js classList: add(...names);remove(...names);toggle(name, force);contains(name);replace(old, new)
js process: argv;env;exit(code);cwd();platform;version;stdout;stdin;nextTick(fn)
js fs: readFileSync(path, encoding);writeFileSync(path, data);existsSync(path);readdirSync(path);mkdirSync(path, options);unlinkSync(path);readFile(path, cb);writeFile(path, data, cb);promises;statSync(path);rmSync(path, options)
js path: join(...parts);resolve(...parts);dirname(path);basename(path, ext);extname(path);relative(from, to);sep;normalize(path)
js *: length;push(...items);pop();shift();unshift(...items);slice(start, end);splice(start, count, ...items);map(fn);filter(fn);forEach(fn);reduce(fn, initial);find(fn);findIndex(fn);findLast(fn);some(fn);every(fn);includes(value);indexOf(value);lastIndexOf(value);join(separator);concat(...items);sort(compare);toSorted(compare);reverse();flat(depth);flatMap(fn);fill(value);at(index);keys();values();entries();split(separator);trim();trimStart();trimEnd();toUpperCase();toLowerCase();startsWith(text);endsWith(text);replace(pattern, replacement);replaceAll(pattern, replacement);padStart(length, fill);padEnd(length, fill);repeat(count);charAt(index);charCodeAt(index);substring(start, end);match(regexp);matchAll(regexp);toString();toFixed(digits);valueOf();then(onFulfilled);catch(onRejected);finally(fn);addEventListener(type, listener);removeEventListener(type, listener);appendChild(node);append(...nodes);prepend(...nodes);remove();replaceWith(...nodes);querySelector(selector);querySelectorAll(selector);setAttribute(name, value);getAttribute(name);removeAttribute(name);hasAttribute(name);closest(selector);focus();blur();click();scrollIntoView(options);getBoundingClientRect();cloneNode(deep);innerHTML;outerHTML;textContent;innerText;value;checked;disabled;style;classList;dataset;id;className;children;parentElement;nextElementSibling;previousElementSibling;firstElementChild;lastElementChild;json();text();blob();ok;status;headers;has(key);get(key);set(key, value);delete(key);clear();size;add(value)
py str: upper();lower();strip(chars);lstrip(chars);rstrip(chars);split(sep);rsplit(sep);splitlines();join(items);replace(old, new);find(sub);rfind(sub);index(sub);count(sub);startswith(prefix);endswith(suffix);format(*args);capitalize();title();swapcase();isdigit();isalpha();isalnum();isspace();isupper();islower();center(width);ljust(width);rjust(width);zfill(width);encode(encoding);partition(sep);removeprefix(prefix);removesuffix(suffix)
py list: append(item);extend(items);insert(index, item);remove(item);pop(index);clear();index(item);count(item);sort(key, reverse);reverse();copy()
py dict: keys();values();items();get(key, default);pop(key);popitem();update(other);setdefault(key, default);clear();copy();fromkeys(keys)
py set: add(item);remove(item);discard(item);pop();clear();union(other);intersection(other);difference(other);symmetric_difference(other);issubset(other);issuperset(other);update(other)
py os: getcwd();listdir(path);mkdir(path);makedirs(path, exist_ok);remove(path);rename(src, dst);rmdir(path);walk(top);system(command);getenv(key, default);environ;path;sep;name;chdir(path);startfile(path);scandir(path)
py os.path: join(*parts);exists(path);isfile(path);isdir(path);basename(path);dirname(path);splitext(path);abspath(path);getsize(path);expanduser(path);split(path);getmtime(path)
py sys: argv;exit(code);path;platform;version;stdin;stdout;stderr;maxsize;executable;modules;setrecursionlimit(limit)
py math: sqrt(x);pow(x, y);floor(x);ceil(x);fabs(x);factorial(n);gcd(a, b);lcm(a, b);log(x, base);log10(x);log2(x);exp(x);sin(x);cos(x);tan(x);asin(x);acos(x);atan(x);atan2(y, x);radians(x);degrees(x);hypot(*coords);isclose(a, b);comb(n, k);perm(n, k);prod(items);fsum(items);trunc(x);pi;e;tau;inf;nan
py random: randint(a, b);random();choice(items);choices(items, k);shuffle(items);sample(items, k);uniform(a, b);randrange(start, stop, step);seed(value);gauss(mu, sigma)
py time: sleep(seconds);time();perf_counter();monotonic();strftime(format);localtime();ctime();gmtime()
py json: loads(text);dumps(obj, indent);load(file);dump(obj, file, indent)
py datetime: datetime;date;time;timedelta;timezone;now();today()
py datetime.datetime: now();today();strptime(text, format);fromtimestamp(ts);fromisoformat(text)
py re: match(pattern, text);search(pattern, text);findall(pattern, text);finditer(pattern, text);sub(pattern, repl, text);split(pattern, text);compile(pattern);fullmatch(pattern, text);escape(text);IGNORECASE;MULTILINE
py turtle: forward(distance);backward(distance);left(angle);right(angle);penup();pendown();goto(x, y);color(color);pencolor(color);fillcolor(color);begin_fill();end_fill();circle(radius);speed(speed);hideturtle();showturtle();shape(name);done();Screen();Turtle();write(text);bgcolor(color);width(width);setheading(angle)
py tkinter: Tk();Label(master, text);Button(master, text, command);Entry(master);Text(master);Frame(master);Canvas(master, width, height);Listbox(master);Scrollbar(master);Checkbutton(master, text);Radiobutton(master, text);Scale(master);messagebox;filedialog;StringVar();IntVar();END;LEFT;RIGHT;TOP;BOTTOM;BOTH;X;Y
py pygame: init();quit();display;event;time;draw;image;font;mixer;key;mouse;Rect(x, y, w, h);Surface(size);QUIT;KEYDOWN;KEYUP;MOUSEBUTTONDOWN
py pygame.display: set_mode(size);set_caption(title);flip();update();get_surface()
py pygame.draw: rect(surface, color, rect);circle(surface, color, center, radius);line(surface, color, start, end);polygon(surface, color, points);ellipse(surface, color, rect)
py subprocess: run(args, capture_output, text);Popen(args);check_output(args);call(args);PIPE
py shutil: copy(src, dst);copytree(src, dst);move(src, dst);rmtree(path);which(cmd)
py pathlib: Path(path)
py collections: Counter(items);defaultdict(factory);deque(items);namedtuple(name, fields);OrderedDict()
py itertools: permutations(items, r);combinations(items, r);product(*items);chain(*items);count(start);cycle(items);groupby(items, key);accumulate(items)
py *: append(item);extend(items);insert(index, item);remove(item);pop(index);clear();index(item);count(item);sort(key, reverse);reverse();copy();keys();values();items();get(key, default);update(other);setdefault(key, default);add(item);discard(item);upper();lower();strip();split(sep);join(items);replace(old, new);find(sub);startswith(prefix);endswith(suffix);format(*args);isdigit();isalpha();read();readline();readlines();write(text);close()
java System: out;in;err;exit(status);currentTimeMillis();nanoTime();getProperty(key);arraycopy(src, srcPos, dest, destPos, length);lineSeparator()
java System.out: println(x);print(x);printf(format, args);flush()
java Math: abs(a);max(a, b);min(a, b);pow(a, b);sqrt(a);random();round(a);floor(a);ceil(a);sin(a);cos(a);tan(a);log(a);log10(a);PI;E
java Integer: parseInt(s);valueOf(s);toString(i);MAX_VALUE;MIN_VALUE;compare(a, b)
java String: valueOf(x);format(format, args);join(delimiter, elements)
java Arrays: sort(a);toString(a);asList(items);fill(a, value);copyOf(a, length);stream(a);binarySearch(a, key);equals(a, b)
java *: length();charAt(index);substring(begin, end);indexOf(s);equals(other);equalsIgnoreCase(other);contains(s);startsWith(prefix);endsWith(suffix);toUpperCase();toLowerCase();trim();split(regex);replace(a, b);isEmpty();toCharArray();add(item);get(index);set(index, item);remove(index);size();contains(item);clear();put(key, value);containsKey(key);keySet();values();entrySet();getOrDefault(key, default);nextLine();nextInt();nextDouble();next();hasNext();hasNextInt();close();stream();forEach(action);toString();hashCode();length
cs Console: WriteLine(value);Write(value);ReadLine();ReadKey();Clear();ForegroundColor;BackgroundColor;ResetColor();Beep();Title
cs Math: Abs(x);Max(a, b);Min(a, b);Pow(x, y);Sqrt(x);Round(x);Floor(x);Ceiling(x);Sin(x);Cos(x);Tan(x);Log(x);Clamp(x, min, max);PI;E
cs int: Parse(s);TryParse(s, out result);MaxValue;MinValue
cs string: Join(separator, values);IsNullOrEmpty(s);IsNullOrWhiteSpace(s);Format(format, args);Concat(values);Empty
cs *: Length;Count;Add(item);Remove(item);RemoveAt(index);Contains(item);Clear();IndexOf(item);Insert(index, item);ToString();ToUpper();ToLower();Trim();Split(separator);Replace(old, new);Substring(start, length);StartsWith(value);EndsWith(value);ToList();ToArray();Where(predicate);Select(selector);First();FirstOrDefault();Any();All(predicate);OrderBy(key);Sum();Max();Min();Average();ContainsKey(key);TryGetValue(key, out value);Keys;Values
go fmt: Println(a...);Printf(format, a...);Print(a...);Sprintf(format, a...);Sprintln(a...);Errorf(format, a...);Scan(a...);Scanln(a...);Scanf(format, a...);Fprintf(w, format, a...);Sscanf(s, format, a...)
go strings: Contains(s, substr);HasPrefix(s, prefix);HasSuffix(s, suffix);Split(s, sep);Join(elems, sep);Replace(s, old, new, n);ReplaceAll(s, old, new);ToUpper(s);ToLower(s);TrimSpace(s);Trim(s, cutset);Index(s, substr);Fields(s);Repeat(s, count);Builder
go strconv: Itoa(i);Atoi(s);ParseFloat(s, bitSize);ParseInt(s, base, bitSize);FormatInt(i, base);Quote(s)
go os: Args;Exit(code);Getenv(key);ReadFile(name);WriteFile(name, data, perm);Open(name);Create(name);Remove(name);Mkdir(name, perm);MkdirAll(path, perm);Stdin;Stdout;Stderr
go math: Sqrt(x);Pow(x, y);Abs(x);Max(x, y);Min(x, y);Floor(x);Ceil(x);Round(x);Sin(x);Cos(x);Log(x);Pi;E;Inf(sign);MaxInt
go time: Now();Sleep(d);Since(t);Second;Millisecond;Minute;Hour;Duration;Tick(d);After(d)
go sort: Ints(x);Strings(x);Slice(x, less);Sort(data);Search(n, f)
rs *: unwrap();expect(msg);unwrap_or(default);unwrap_or_default();clone();to_string();to_owned();as_str();len();is_empty();push(value);pop();insert(index, value);remove(index);contains(value);iter();iter_mut();into_iter();map(f);filter(f);collect();enumerate();rev();sum();count();min();max();find(f);any(f);all(f);fold(init, f);for_each(f);zip(other);skip(n);take(n);chars();bytes();lines();split(pat);trim();parse();get(key);get_mut(key);entry(key);or_insert(default);keys();values();sort();sort_by(f);dedup();extend(iter);join(sep);starts_with(pat);ends_with(pat);replace(from, to);to_uppercase();to_lowercase();is_some();is_none();is_ok();is_err();ok();err();and_then(f);map_err(f);as_ref();borrow();lock()
rs String: new();from(s);with_capacity(n);from_utf8(bytes)
rs Vec: new();with_capacity(n);from(items)
rs HashMap: new();with_capacity(n);from(items)
rs std: io;fs;env;collections;thread;time;process;fmt;path;sync
rs io: stdin();stdout();Write;Read;BufRead;Result;Error
rs fs: read_to_string(path);write(path, contents);read(path);create_dir_all(path);remove_file(path);read_dir(path);copy(from, to);rename(from, to)
cpp std: cout;cin;cerr;endl;string;vector;map;unordered_map;set;unordered_set;pair;make_pair(a, b);sort(first, last);reverse(first, last);max(a, b);min(a, b);swap(a, b);to_string(value);stoi(s);stod(s);getline(in, str);find(first, last, value);accumulate(first, last, init);array;queue;stack;deque;priority_queue;unique_ptr;shared_ptr;make_unique();make_shared();optional;tuple;abs(x);sqrt(x);pow(x, y)
cpp *: push_back(value);emplace_back(args);pop_back();size();empty();clear();begin();end();front();back();at(index);insert(pos, value);erase(pos);find(key);count(key);substr(pos, len);length();c_str();append(str);resize(n);reserve(n);first;second
";

// moduly pre „import …“
pub const PY_MODULES: &str = "os sys math random time json datetime re turtle tkinter pygame subprocess shutil pathlib collections itertools functools typing dataclasses string csv sqlite3 urllib http socket threading asyncio logging argparse copy pickle statistics decimal fractions hashlib base64 uuid glob tempfile unittest requests numpy pandas matplotlib flask";
pub const JS_MODULES: &str = "fs path os http https url crypto child_process events util readline stream zlib express react react-dom vue";
