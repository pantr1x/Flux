# Flux 1.4.8: premenovanie všade – aj v ostatných súboroch a cesty po premenovaní priečinka (sk, de, es, fr, it, pl, pt, uk).
_V17 = {
    'Also in 1 other file': ('Aj v 1 ďalšom súbore', 'Auch in 1 weiteren Datei', 'También en 1 archivo más', 'Aussi dans 1 autre fichier', 'Anche in 1 altro file', 'Także w 1 innym pliku', 'Também em mais 1 arquivo', 'Також в 1 іншому файлі'),
    'Also in {n} other files': ('Aj v ďalších súboroch: {n}', 'Auch in {n} weiteren Dateien', 'También en {n} archivos más', 'Aussi dans {n} autres fichiers', 'Anche in altri {n} file', 'Także w innych plikach: {n}', 'Também em mais {n} arquivos', 'Також в інших файлах: {n}'),
    'Changed {n} places in 1 other file.': (
        'Zmenené miesta v 1 ďalšom súbore: {n}.',
        '{n} Stellen in 1 weiteren Datei geändert.',
        'Se cambiaron {n} lugares en 1 archivo más.',
        '{n} endroits modifiés dans 1 autre fichier.',
        '{n} punti modificati in 1 altro file.',
        'Zmieniono miejsca w 1 innym pliku: {n}.',
        '{n} lugares alterados em mais 1 arquivo.',
        'Змінено місць в 1 іншому файлі: {n}.'),
    'Changed {n} places in {f} other files.': (
        'Zmenené miesta: {n} v ďalších súboroch ({f}).',
        '{n} Stellen in {f} weiteren Dateien geändert.',
        'Se cambiaron {n} lugares en {f} archivos más.',
        '{n} endroits modifiés dans {f} autres fichiers.',
        '{n} punti modificati in altri {f} file.',
        'Zmieniono miejsca: {n} w innych plikach ({f}).',
        '{n} lugares alterados em mais {f} arquivos.',
        'Змінено місць: {n} в інших файлах ({f}).'),
    'Update {n} paths to {name} in 1 file?': (
        'Upraviť cesty k {name} v 1 súbore ({n})?',
        '{n} Pfade zu {name} in 1 Datei anpassen?',
        '¿Actualizar {n} rutas a {name} en 1 archivo?',
        'Mettre à jour {n} chemins vers {name} dans 1 fichier ?',
        'Aggiornare {n} percorsi a {name} in 1 file?',
        'Zaktualizować ścieżki do {name} w 1 pliku ({n})?',
        'Atualizar {n} caminhos para {name} em 1 arquivo?',
        'Оновити шляхи до {name} в 1 файлі ({n})?'),
    'Update {n} paths to {name} in {f} files?': (
        'Upraviť cesty k {name} ({n}) v súboroch: {f}?',
        '{n} Pfade zu {name} in {f} Dateien anpassen?',
        '¿Actualizar {n} rutas a {name} en {f} archivos?',
        'Mettre à jour {n} chemins vers {name} dans {f} fichiers ?',
        'Aggiornare {n} percorsi a {name} in {f} file?',
        'Zaktualizować ścieżki do {name} ({n}) w plikach: {f}?',
        'Atualizar {n} caminhos para {name} em {f} arquivos?',
        'Оновити шляхи до {name} ({n}) у файлах: {f}?'),
    'Updated {n} paths.': ('Upravené cesty: {n}.', '{n} Pfade angepasst.', 'Se actualizaron {n} rutas.', '{n} chemins mis à jour.', '{n} percorsi aggiornati.', 'Zaktualizowano ścieżki: {n}.', '{n} caminhos atualizados.', 'Оновлено шляхів: {n}.'),
}
V17 = {code: {k: v[i] for k, v in _V17.items()} for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk'])}
