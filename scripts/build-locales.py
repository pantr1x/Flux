# Zostaví locales/*.json z prekladov a skontroluje, či sedia zástupné znaky {…}.
import json, re, sys, os
sys.path.insert(0, os.path.dirname(__file__))
from locales_sk import SK
from locales_de import DE
from locales_es import ES
from locales_v3 import V3
from locales_v4 import V4
from locales_v5 import V5
from locales_v6 import V6
from locales_v7 import V7
from locales_v8 import V8
from locales_fip_a import FIP_A
from locales_fip_b import FIP_B
from locales_ptuk import PT, UK
from locales_v9 import V9
from locales_v10 import V10
from locales_v11 import V11
from locales_v12 import V12
from locales_v13 import V13
from locales_v14 import V14
from locales_v15 import V15
from locales_v16 import V16
from locales_v17 import V17
from locales_v18 import V18
from locales_v19 import V19
from locales_v20 import V20
from locales_v21 import V21
from locales_v22 import V22
from locales_v23 import V23
from locales_v24 import V24
from locales_v25 import V25
from locales_v26 import V26
from locales_v27 import V27
from locales_v28 import V28
from locales_v29 import V29
from locales_v30 import V30
from locales_v31 import V31
from locales_v32 import V32
from locales_v33 import V33
from locales_v34 import V34
from locales_v35 import V35
from locales_v36 import V36
from locales_v37 import V37
from locales_v38 import V38
from locales_v39 import V39
FIP = {**FIP_A, **FIP_B}
FR, IT, PL = ({k: v[i] for k, v in FIP.items()} for i in range(3))
keys = json.load(open('locales/en.keys.json'))
langs = {'sk': ('Slovak', 'Slovenčina', SK), 'de': ('German', 'Deutsch', DE), 'es': ('Spanish', 'Español', ES), 'fr': ('French', 'Français', FR), 'it': ('Italian', 'Italiano', IT), 'pl': ('Polish', 'Polski', PL), 'pt': ('Portuguese', 'Português', PT), 'uk': ('Ukrainian', 'Українська', UK)}
ph = lambda s: sorted(re.findall(r'\{\w+\}', s))
index = [{'code': 'en', 'name': 'English', 'native': 'English'}]
for code, (name, native, d) in langs.items():
    d = {**d, **V3.get(code, {}), **V4.get(code, {}), **V5.get(code, {}), **V6.get(code, {}), **V7.get(code, {}), **V8.get(code, {}), **V9.get(code, {}), **V10.get(code, {}), **V11.get(code, {}), **V12.get(code, {}), **V13.get(code, {}), **V14.get(code, {}), **V15.get(code, {}), **V16.get(code, {}), **V17.get(code, {}), **V18.get(code, {}), **V19.get(code, {}), **V20.get(code, {}), **V21.get(code, {}), **V22.get(code, {}), **V23.get(code, {}), **V24.get(code, {}), **V25.get(code, {}), **V26.get(code, {}), **V27.get(code, {}), **V28.get(code, {}), **V29.get(code, {}), **V30.get(code, {}), **V31.get(code, {}), **V32.get(code, {}), **V33.get(code, {}), **V34.get(code, {}), **V35.get(code, {}), **V36.get(code, {}), **V37.get(code, {}), **V38.get(code, {}), **V39.get(code, {})}
    out = {}
    for k, v in d.items():
        if ph(k) != ph(v):
            print(f'[{code}] placeholder mismatch: {k!r} -> {v!r}')
        out[k] = v
    missing = [k for k in keys if k not in d and re.search('[A-Za-z]{3}', k) and not re.match(r'^(if|for|fore|def|class|main|ifmain|ifelse|input|inputint|printf|lambda|listcomp|try|while|with|Flux|English|xterm.*|python -m venv.*|\[.*|Git|Java|Lua|PHP|Ruby|Rust|Claude .*|flux_\w+|main\.\w+|index\.php|Program\.cs.*|input\(\) \+ print\(\)|if / else|if __name__.*|\.css)$', k)]
    json.dump(out, open(f'locales/{code}.json', 'w'), ensure_ascii=False, indent=1, sort_keys=True)
    print(code, len(out), 'preložených, chýba', len(missing), missing[:8] if len(sys.argv) < 2 else missing)
    index.append({'code': code, 'name': name, 'native': native})
json.dump(index, open('locales/index.json', 'w'), ensure_ascii=False, indent=1)
