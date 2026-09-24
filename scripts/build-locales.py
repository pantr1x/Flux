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
from locales_fip_a import FIP_A
from locales_fip_b import FIP_B
FIP = {**FIP_A, **FIP_B}
FR, IT, PL = ({k: v[i] for k, v in FIP.items()} for i in range(3))
keys = json.load(open('locales/en.keys.json'))
langs = {'sk': ('Slovak', 'Slovenčina', SK), 'de': ('German', 'Deutsch', DE), 'es': ('Spanish', 'Español', ES), 'fr': ('French', 'Français', FR), 'it': ('Italian', 'Italiano', IT), 'pl': ('Polish', 'Polski', PL)}
ph = lambda s: sorted(re.findall(r'\{\w+\}', s))
index = [{'code': 'en', 'name': 'English', 'native': 'English'}]
for code, (name, native, d) in langs.items():
    d = {**d, **V3.get(code, {}), **V4.get(code, {}), **V5[code], **V6[code], **V7[code]}
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
