# Flux Native 0.9.74: pin nastavený len cez pinMode() = nepoužitý.
# Poradie: sk, de, es, fr, it, pl, pt, uk.
_V40 = {
    'Pin {pin} is set up with pinMode() but nothing in the code uses it.': ('Pin {pin} je nastavený cez pinMode(), ale nič v kóde ho nepoužíva.', 'Pin {pin} ist mit pinMode() eingerichtet, aber nichts im Code nutzt ihn.', 'El pin {pin} está configurado con pinMode(), pero nada en el código lo usa.', 'La broche {pin} est configurée avec pinMode(), mais rien dans le code ne l’utilise.', 'Il pin {pin} è impostato con pinMode(), ma nel codice nulla lo usa.', 'Pin {pin} jest ustawiony przez pinMode(), ale nic w kodzie go nie używa.', 'O pino {pin} está configurado com pinMode(), mas nada no código o usa.', 'Пін {pin} налаштовано через pinMode(), але в коді його ніщо не використовує.'),
}
V40 = {code: {k: v[i] for k, v in _V40.items()} for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk'])}
