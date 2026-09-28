# Preklady pre 1.4.14 – súbor zmenený mimo Fluxu. Poradie: sk, de, es, fr, it, pl, pt, uk.
_V19 = {
    '“{file}” was changed outside Flux, and you have unsaved changes.': (
        '„{file}“ sa zmenil mimo Fluxu a ty máš neuložené zmeny.',
        '„{file}“ wurde außerhalb von Flux geändert, und du hast ungespeicherte Änderungen.',
        '«{file}» se cambió fuera de Flux y tienes cambios sin guardar.',
        '« {file} » a été modifié en dehors de Flux, et tu as des modifications non enregistrées.',
        '«{file}» è stato modificato fuori da Flux e hai modifiche non salvate.',
        '„{file}” zmienił się poza Fluxem, a ty masz niezapisane zmiany.',
        '“{file}” foi alterado fora do Flux e você tem alterações não salvas.',
        '«{file}» змінено поза Flux, а в тебе є незбережені зміни.'),
    'Load the new version from disk': (
        'Načítať novú verziu z disku', 'Neue Version von der Festplatte laden', 'Cargar la nueva versión del disco',
        'Charger la nouvelle version depuis le disque', 'Carica la nuova versione dal disco', 'Wczytaj nową wersję z dysku',
        'Carregar a nova versão do disco', 'Завантажити нову версію з диска'),
    'Keep my changes': (
        'Ponechať moje zmeny', 'Meine Änderungen behalten', 'Mantener mis cambios', 'Garder mes modifications',
        'Mantieni le mie modifiche', 'Zachowaj moje zmiany', 'Manter minhas alterações', 'Залишити мої зміни'),
    '“{file}” was reloaded from disk.': (
        '„{file}“ sa načítal znova z disku.', '„{file}“ wurde neu von der Festplatte geladen.', '«{file}» se recargó desde el disco.',
        '« {file} » a été rechargé depuis le disque.', '«{file}» è stato ricaricato dal disco.', '„{file}” wczytano ponownie z dysku.',
        '“{file}” foi recarregado do disco.', '«{file}» перезавантажено з диска.'),
    '“{file}” changed on disk – the editor now shows the new version.': (
        '„{file}“ sa zmenil na disku – editor už ukazuje novú verziu.',
        '„{file}“ hat sich auf der Festplatte geändert – der Editor zeigt jetzt die neue Version.',
        '«{file}» cambió en el disco: el editor ya muestra la nueva versión.',
        '« {file} » a changé sur le disque – l’éditeur affiche maintenant la nouvelle version.',
        '«{file}» è cambiato sul disco – l’editor ora mostra la nuova versione.',
        '„{file}” zmienił się na dysku – edytor pokazuje już nową wersję.',
        '“{file}” mudou no disco – o editor já mostra a nova versão.',
        '«{file}» змінився на диску – редактор уже показує нову версію.'),
    'Keep my old version': (
        'Nechať starú verziu', 'Alte Version behalten', 'Mantener la versión anterior', 'Garder l’ancienne version',
        'Mantieni la versione precedente', 'Zostaw starą wersję', 'Manter a versão antiga', 'Залишити стару версію'),
}
V19 = {code: {k: v[i] for k, v in _V19.items()} for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk'])}

# 1.4.16 – otázka pred načítaním zmeneného súboru.
_V19B = {
    '“{file}” was changed outside Flux. Show the new version?': (
        '„{file}“ sa zmenil mimo Fluxu. Zobraziť novú verziu?',
        '„{file}“ wurde außerhalb von Flux geändert. Neue Version anzeigen?',
        '«{file}» se cambió fuera de Flux. ¿Mostrar la nueva versión?',
        '« {file} » a été modifié en dehors de Flux. Afficher la nouvelle version ?',
        '«{file}» è stato modificato fuori da Flux. Mostrare la nuova versione?',
        '„{file}” zmienił się poza Fluxem. Pokazać nową wersję?',
        '“{file}” foi alterado fora do Flux. Mostrar a nova versão?',
        '«{file}» змінено поза Flux. Показати нову версію?'),
    'Load the new version': (
        'Načítať novú verziu', 'Neue Version laden', 'Cargar la nueva versión', 'Charger la nouvelle version',
        'Carica la nuova versione', 'Wczytaj nową wersję', 'Carregar a nova versão', 'Завантажити нову версію'),
    'Keep what is in the editor': (
        'Nechať to, čo je v editore', 'Behalten, was im Editor steht', 'Mantener lo que hay en el editor', 'Garder ce qui est dans l’éditeur',
        'Mantieni ciò che c’è nell’editor', 'Zostaw to, co jest w edytorze', 'Manter o que está no editor', 'Залишити те, що в редакторі'),
    'Always load automatically – don’t ask again': (
        'Vždy načítať automaticky – už sa nepýtať', 'Immer automatisch laden – nicht mehr fragen', 'Cargar siempre automáticamente: no volver a preguntar',
        'Toujours charger automatiquement – ne plus demander', 'Carica sempre automaticamente – non chiedere più', 'Zawsze wczytuj automatycznie – nie pytaj więcej',
        'Carregar sempre automaticamente – não perguntar de novo', 'Завжди завантажувати автоматично – більше не питати'),
    'Changed files now load automatically. You can turn this off in Settings → Editor.': (
        'Zmenené súbory sa teraz načítajú automaticky. Vypneš to v Nastavenia → Editor.',
        'Geänderte Dateien werden jetzt automatisch geladen. Ausschalten unter Einstellungen → Editor.',
        'Los archivos cambiados ahora se cargan automáticamente. Puedes desactivarlo en Ajustes → Editor.',
        'Les fichiers modifiés se chargent maintenant automatiquement. Désactivable dans Réglages → Éditeur.',
        'I file modificati ora si caricano automaticamente. Puoi disattivarlo in Impostazioni → Editor.',
        'Zmienione pliki wczytują się teraz automatycznie. Wyłączysz to w Ustawienia → Edytor.',
        'Os arquivos alterados agora carregam automaticamente. Desative em Configurações → Editor.',
        'Змінені файли тепер завантажуються автоматично. Вимкнути можна в Налаштування → Редактор.'),
    'Reload changed files without asking': (
        'Načítať zmenené súbory bez pýtania', 'Geänderte Dateien ohne Nachfrage neu laden', 'Recargar archivos cambiados sin preguntar',
        'Recharger les fichiers modifiés sans demander', 'Ricarica i file modificati senza chiedere', 'Wczytuj zmienione pliki bez pytania',
        'Recarregar arquivos alterados sem perguntar', 'Перезавантажувати змінені файли без запиту'),
    'when another program changes an open file, the editor shows the new version right away': (
        'keď iný program zmení otvorený súbor, editor hneď ukáže novú verziu',
        'wenn ein anderes Programm eine offene Datei ändert, zeigt der Editor sofort die neue Version',
        'cuando otro programa cambia un archivo abierto, el editor muestra la nueva versión al instante',
        'quand un autre programme modifie un fichier ouvert, l’éditeur affiche tout de suite la nouvelle version',
        'quando un altro programma modifica un file aperto, l’editor mostra subito la nuova versione',
        'gdy inny program zmieni otwarty plik, edytor od razu pokaże nową wersję',
        'quando outro programa altera um arquivo aberto, o editor mostra a nova versão na hora',
        'коли інша програма змінює відкритий файл, редактор одразу показує нову версію'),
}
for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk']):
    V19[code].update({k: v[i] for k, v in _V19B.items()})
