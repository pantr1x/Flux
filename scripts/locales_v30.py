# Flux Native 0.9.64: tmavý dashboard dosky na stránke projektu.
# Poradie: sk, de, es, fr, it, pl, pt, uk.
_V30 = {
    'BOARD': ('DOSKA', 'BOARD', 'PLACA', 'CARTE', 'SCHEDA', 'PŁYTKA', 'PLACA', 'ПЛАТА'),
    'CUSTOM CODE': ('VLASTNÝ KÓD', 'EIGENER CODE', 'CÓDIGO PROPIO', 'CODE PERSONNALISÉ', 'CODICE PERSONALIZZATO', 'WŁASNY KOD', 'CÓDIGO PRÓPRIO', 'ВЛАСНИЙ КОД'),
}
V30 = {code: {k: v[i] for k, v in _V30.items()} for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk'])}
