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

# 1.4.19 – ukladanie cez súbor zmenený zvonku.
_V19C = {
    '“{file}” was changed outside Flux. Save your version over it?': (
        '„{file}“ sa zmenil mimo Fluxu. Uložiť cez neho tvoju verziu?',
        '„{file}“ wurde außerhalb von Flux geändert. Deine Version darüber speichern?',
        '«{file}» se cambió fuera de Flux. ¿Guardar tu versión encima?',
        '« {file} » a été modifié en dehors de Flux. Enregistrer ta version par-dessus ?',
        '«{file}» è stato modificato fuori da Flux. Salvare la tua versione sopra?',
        '„{file}” zmienił się poza Fluxem. Zapisać na nim twoją wersję?',
        '“{file}” foi alterado fora do Flux. Salvar a sua versão por cima?',
        '«{file}» змінено поза Flux. Зберегти поверх нього твою версію?'),
    'Save my version over it': (
        'Uložiť moju verziu cez neho', 'Meine Version darüber speichern', 'Guardar mi versión encima', 'Enregistrer ma version par-dessus',
        'Salva la mia versione sopra', 'Zapisz moją wersję na nim', 'Salvar a minha versão por cima', 'Зберегти мою версію поверх'),
}
for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk']):
    V19[code].update({k: v[i] for k, v in _V19C.items()})

# 1.4.20 – okno s otázkou.
_V19D = {
    'File changed outside Flux': (
        'Súbor sa zmenil mimo Fluxu', 'Datei außerhalb von Flux geändert', 'Archivo cambiado fuera de Flux', 'Fichier modifié en dehors de Flux',
        'File modificato fuori da Flux', 'Plik zmieniony poza Fluxem', 'Arquivo alterado fora do Flux', 'Файл змінено поза Flux'),
}
for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk']):
    V19[code].update({k: v[i] for k, v in _V19D.items()})

# 1.4.24 – upozornenie pred inštaláciou podozrivého pluginu.
_V19E = {
    'Check this plugin before installing': ('Pred inštaláciou tento plugin skontroluj', 'Prüfe dieses Plugin vor der Installation', 'Revisa este plugin antes de instalarlo', 'Vérifie ce plugin avant de l’installer', 'Controlla questo plugin prima di installarlo', 'Sprawdź tę wtyczkę przed instalacją', 'Verifique este plugin antes de instalar', 'Перевір цей плагін перед встановленням'),
    'Flux found code in this plugin that could harm your PC or steal your data:': (
        'Flux našiel v tomto plugine kód, ktorý môže poškodiť tvoj počítač alebo ukradnúť tvoje dáta:',
        'Flux hat in diesem Plugin Code gefunden, der deinem PC schaden oder deine Daten stehlen könnte:',
        'Flux encontró código en este plugin que podría dañar tu PC o robar tus datos:',
        'Flux a trouvé dans ce plugin du code qui pourrait endommager ton PC ou voler tes données :',
        'Flux ha trovato in questo plugin codice che potrebbe danneggiare il tuo PC o rubare i tuoi dati:',
        'Flux znalazł w tej wtyczce kod, który może uszkodzić twój komputer lub ukraść twoje dane:',
        'O Flux encontrou neste plugin código que pode danificar o seu PC ou roubar os seus dados:',
        'Flux знайшов у цьому плагіні код, який може зашкодити твоєму ПК або вкрасти твої дані:'),
    'Install anyway': ('Aj tak nainštalovať', 'Trotzdem installieren', 'Instalar de todos modos', 'Installer quand même', 'Installa comunque', 'Zainstaluj mimo to', 'Instalar mesmo assim', 'Все одно встановити'),
    'Install it only if you trust its author.': ('Nainštaluj ho, len ak jeho autorovi dôveruješ.', 'Installiere es nur, wenn du dem Autor vertraust.', 'Instálalo solo si confías en su autor.', 'Installe-le seulement si tu fais confiance à son auteur.', 'Installalo solo se ti fidi del suo autore.', 'Zainstaluj ją tylko, jeśli ufasz jej autorowi.', 'Instale apenas se confiar no autor.', 'Встановлюй його, лише якщо довіряєш автору.'),
    'This plugin does things you should know about:': ('Tento plugin robí veci, o ktorých by si mal vedieť:', 'Dieses Plugin macht Dinge, die du wissen solltest:', 'Este plugin hace cosas que deberías saber:', 'Ce plugin fait des choses que tu devrais savoir :', 'Questo plugin fa cose che dovresti sapere:', 'Ta wtyczka robi rzeczy, o których warto wiedzieć:', 'Este plugin faz coisas que você deveria saber:', 'Цей плагін робить речі, про які тобі варто знати:'),
    'This plugin may be dangerous': ('Tento plugin môže byť nebezpečný', 'Dieses Plugin könnte gefährlich sein', 'Este plugin puede ser peligroso', 'Ce plugin peut être dangereux', 'Questo plugin potrebbe essere pericoloso', 'Ta wtyczka może być niebezpieczna', 'Este plugin pode ser perigoso', 'Цей плагін може бути небезпечним'),
    'adds a script to the window': ('pridáva do okna skript', 'fügt dem Fenster ein Skript hinzu', 'añade un script a la ventana', 'ajoute un script à la fenêtre', 'aggiunge uno script alla finestra', 'dodaje skrypt do okna', 'adiciona um script à janela', 'додає скрипт у вікно'),
    'connects to the internet': ('pripája sa na internet', 'verbindet sich mit dem Internet', 'se conecta a internet', 'se connecte à internet', 'si connette a internet', 'łączy się z internetem', 'conecta-se à internet', 'підключається до інтернету'),
    'contains hidden (encoded) text': ('obsahuje skrytý (zakódovaný) text', 'enthält versteckten (kodierten) Text', 'contiene texto oculto (codificado)', 'contient du texte caché (encodé)', 'contiene testo nascosto (codificato)', 'zawiera ukryty (zakodowany) tekst', 'contém texto oculto (codificado)', 'містить прихований (закодований) текст'),
    'decodes hidden text': ('dekóduje skrytý text', 'entschlüsselt versteckten Text', 'decodifica texto oculto', 'décode du texte caché', 'decodifica testo nascosto', 'dekoduje ukryty tekst', 'decodifica texto oculto', 'декодує прихований текст'),
    'downloads and runs code from the internet': ('sťahuje a spúšťa kód z internetu', 'lädt Code aus dem Internet und führt ihn aus', 'descarga y ejecuta código de internet', 'télécharge et exécute du code depuis internet', 'scarica ed esegue codice da internet', 'pobiera i uruchamia kod z internetu', 'baixa e executa código da internet', 'завантажує й запускає код з інтернету'),
    'image with hidden code (SVG script)': ('obrázok so skrytým kódom (skript v SVG)', 'Bild mit verstecktem Code (SVG-Skript)', 'imagen con código oculto (script SVG)', 'image avec du code caché (script SVG)', 'immagine con codice nascosto (script SVG)', 'obraz z ukrytym kodem (skrypt SVG)', 'imagem com código oculto (script SVG)', 'зображення з прихованим кодом (скрипт SVG)'),
    'minified or hidden code (very long line)': ('zhustený alebo skrytý kód (veľmi dlhý riadok)', 'minifizierter oder versteckter Code (sehr lange Zeile)', 'código minificado u oculto (línea muy larga)', 'code minifié ou caché (ligne très longue)', 'codice minificato o nascosto (riga molto lunga)', 'zminifikowany lub ukryty kod (bardzo długa linia)', 'código minificado ou oculto (linha muito longa)', 'стиснутий або прихований код (дуже довгий рядок)'),
    'reads or writes browser storage': ('číta alebo zapisuje úložisko prehliadača', 'liest oder schreibt den Browser-Speicher', 'lee o escribe el almacenamiento del navegador', 'lit ou écrit le stockage du navigateur', 'legge o scrive la memoria del browser', 'czyta lub zapisuje pamięć przeglądarki', 'lê ou grava o armazenamento do navegador', 'читає або записує сховище браузера'),
    'runs text as code (eval)': ('spúšťa text ako kód (eval)', 'führt Text als Code aus (eval)', 'ejecuta texto como código (eval)', 'exécute du texte comme du code (eval)', 'esegue testo come codice (eval)', 'uruchamia tekst jako kod (eval)', 'executa texto como código (eval)', 'виконує текст як код (eval)'),
    'runs text as code (new Function)': ('spúšťa text ako kód (new Function)', 'führt Text als Code aus (new Function)', 'ejecuta texto como código (new Function)', 'exécute du texte comme du code (new Function)', 'esegue testo come codice (new Function)', 'uruchamia tekst jako kod (new Function)', 'executa texto como código (new Function)', 'виконує текст як код (new Function)'),
    'uses Node.js or Electron (full access to your PC)': ('používa Node.js alebo Electron (plný prístup k počítaču)', 'nutzt Node.js oder Electron (voller Zugriff auf deinen PC)', 'usa Node.js o Electron (acceso total a tu PC)', 'utilise Node.js ou Electron (accès complet à ton PC)', 'usa Node.js o Electron (accesso completo al PC)', 'używa Node.js lub Electrona (pełny dostęp do komputera)', 'usa Node.js ou Electron (acesso total ao PC)', 'використовує Node.js або Electron (повний доступ до ПК)'),
    'uses the Flux app bridge directly (files and programs on your PC)': ('priamo používa most aplikácie Flux (súbory a programy v počítači)', 'nutzt direkt die Flux-App-Brücke (Dateien und Programme auf deinem PC)', 'usa directamente el puente de la app Flux (archivos y programas de tu PC)', 'utilise directement le pont de l’app Flux (fichiers et programmes de ton PC)', 'usa direttamente il ponte dell’app Flux (file e programmi del PC)', 'używa bezpośrednio mostu aplikacji Flux (pliki i programy na komputerze)', 'usa diretamente a ponte do app Flux (arquivos e programas do PC)', 'напряму використовує міст застосунку Flux (файли й програми на ПК)'),
    '…and {n} more': ('…a ďalšie ({n})', '…und {n} weitere', '…y {n} más', '…et {n} de plus', '…e altri {n}', '…i jeszcze {n}', '…e mais {n}', '…і ще {n}'),
}
for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk']):
    V19[code].update({k: v[i] for k, v in _V19E.items()})

# 1.4.27 – pozadie z videa / YouTube, história pozadí.
_V19F = {
    'Background image or video': ('Obrázok alebo video na pozadie', 'Hintergrundbild oder -video', 'Imagen o vídeo de fondo', 'Image ou vidéo de fond', 'Immagine o video di sfondo', 'Obraz lub wideo w tle', 'Imagem ou vídeo de fundo', 'Зображення або відео для фону'),
    'Image or video…': ('Obrázok alebo video…', 'Bild oder Video…', 'Imagen o vídeo…', 'Image ou vidéo…', 'Immagine o video…', 'Obraz lub wideo…', 'Imagem ou vídeo…', 'Зображення або відео…'),
    'Images and videos': ('Obrázky a videá', 'Bilder und Videos', 'Imágenes y vídeos', 'Images et vidéos', 'Immagini e video', 'Obrazy i wideo', 'Imagens e vídeos', 'Зображення та відео'),
    'Paste a YouTube link, e.g. https://youtu.be/…': ('Vlož odkaz na YouTube, napr. https://youtu.be/…', 'Füge einen YouTube-Link ein, z. B. https://youtu.be/…', 'Pega un enlace de YouTube, p. ej. https://youtu.be/…', 'Colle un lien YouTube, par ex. https://youtu.be/…', 'Incolla un link di YouTube, ad es. https://youtu.be/…', 'Wklej link do YouTube, np. https://youtu.be/…', 'Cole um link do YouTube, ex. https://youtu.be/…', 'Встав посилання на YouTube, напр. https://youtu.be/…'),
    'Previous backgrounds': ('Predošlé pozadia', 'Frühere Hintergründe', 'Fondos anteriores', 'Fonds précédents', 'Sfondi precedenti', 'Poprzednie tła', 'Fundos anteriores', 'Попередні фони'),
    'The file is too large (over 400 MB).': ('Súbor je príliš veľký (viac ako 400 MB).', 'Die Datei ist zu groß (über 400 MB).', 'El archivo es demasiado grande (más de 400 MB).', 'Le fichier est trop volumineux (plus de 400 Mo).', 'Il file è troppo grande (oltre 400 MB).', 'Plik jest za duży (ponad 400 MB).', 'O arquivo é grande demais (mais de 400 MB).', 'Файл завеликий (понад 400 МБ).'),
    'The video plays muted in the background, blurred like the wallpaper.': ('Video hrá na pozadí bez zvuku, rozmazané ako tapeta.', 'Das Video läuft stumm im Hintergrund, unscharf wie das Hintergrundbild.', 'El vídeo se reproduce sin sonido en el fondo, desenfocado como el fondo de pantalla.', 'La vidéo tourne sans son en arrière-plan, floutée comme le fond d’écran.', 'Il video scorre senza audio sullo sfondo, sfocato come lo sfondo.', 'Wideo odtwarza się bez dźwięku w tle, rozmyte jak tapeta.', 'O vídeo toca sem som no fundo, desfocado como o papel de parede.', 'Відео грає без звуку на фоні, розмите як шпалери.'),
    'This is not a YouTube link.': ('Toto nie je odkaz na YouTube.', 'Das ist kein YouTube-Link.', 'Esto no es un enlace de YouTube.', 'Ce n’est pas un lien YouTube.', 'Questo non è un link di YouTube.', 'To nie jest link do YouTube.', 'Isto não é um link do YouTube.', 'Це не посилання на YouTube.'),
    'Videos': ('Videá', 'Videos', 'Vídeos', 'Vidéos', 'Video', 'Wideo', 'Vídeos', 'Відео'),
    'Windows wallpaper': ('Tapeta Windows', 'Windows-Hintergrund', 'Fondo de Windows', 'Fond d’écran Windows', 'Sfondo di Windows', 'Tapeta Windows', 'Papel de parede do Windows', 'Шпалери Windows'),
    'click one to use it again': ('kliknutím ho znova použiješ', 'klicke auf einen, um ihn wieder zu nutzen', 'haz clic en uno para usarlo de nuevo', 'clique sur un fond pour le réutiliser', 'fai clic su uno per usarlo di nuovo', 'kliknij, aby użyć ponownie', 'clique em um para usar de novo', 'натисни, щоб використати знову'),
    'your own picture or video, or a YouTube video, instead of the Windows wallpaper': ('vlastný obrázok, video alebo video z YouTube namiesto tapety Windows', 'eigenes Bild, Video oder YouTube-Video statt des Windows-Hintergrunds', 'tu imagen, vídeo o un vídeo de YouTube en lugar del fondo de Windows', 'ta propre image, vidéo ou une vidéo YouTube au lieu du fond Windows', 'la tua immagine, un video o un video di YouTube al posto dello sfondo di Windows', 'własny obraz, wideo lub film z YouTube zamiast tapety Windows', 'sua imagem, vídeo ou um vídeo do YouTube em vez do papel de parede do Windows', 'власне зображення, відео або відео з YouTube замість шпалер Windows'),
    'Remove from the list': ('Odstrániť zo zoznamu', 'Aus der Liste entfernen', 'Quitar de la lista', 'Retirer de la liste', 'Rimuovi dall’elenco', 'Usuń z listy', 'Remover da lista', 'Прибрати зі списку'),
}
for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk']):
    V19[code].update({k: v[i] for k, v in _V19F.items()})

# 1.4.29 – Lively Wallpaper / Wallpaper Engine.
_V19G = {
    'Use Lively Wallpaper and Wallpaper Engine': ('Použiť Lively Wallpaper a Wallpaper Engine', 'Lively Wallpaper und Wallpaper Engine verwenden', 'Usar Lively Wallpaper y Wallpaper Engine', 'Utiliser Lively Wallpaper et Wallpaper Engine', 'Usa Lively Wallpaper e Wallpaper Engine', 'Używaj Lively Wallpaper i Wallpaper Engine', 'Usar Lively Wallpaper e Wallpaper Engine', 'Використовувати Lively Wallpaper і Wallpaper Engine'),
    'when one of them is running and you have no own background, Flux shows the same live wallpaper': (
        'keď jeden z nich beží a nemáš vlastné pozadie, Flux ukáže tú istú živú tapetu',
        'wenn eines davon läuft und du keinen eigenen Hintergrund hast, zeigt Flux dasselbe Live-Hintergrundbild',
        'cuando uno de ellos está en marcha y no tienes fondo propio, Flux muestra el mismo fondo animado',
        'quand l’un d’eux tourne et que tu n’as pas de fond personnel, Flux affiche le même fond animé',
        'quando uno dei due è in esecuzione e non hai uno sfondo tuo, Flux mostra lo stesso sfondo animato',
        'gdy jeden z nich działa, a nie masz własnego tła, Flux pokazuje tę samą animowaną tapetę',
        'quando um deles está rodando e você não tem fundo próprio, o Flux mostra o mesmo papel de parede animado',
        'коли одна з них працює і ти не маєш власного фону, Flux показує ті самі живі шпалери'),
    'Now: {source}': ('Teraz: {source}', 'Jetzt: {source}', 'Ahora: {source}', 'Maintenant : {source}', 'Ora: {source}', 'Teraz: {source}', 'Agora: {source}', 'Зараз: {source}'),
}
for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk']):
    V19[code].update({k: v[i] for k, v in _V19G.items()})

# 1.4.31 – farby podľa pozadia.
_V19H = {
    'Colors follow the background': ('Farby podľa pozadia', 'Farben passen sich dem Hintergrund an', 'Los colores siguen al fondo', 'Les couleurs suivent le fond', 'I colori seguono lo sfondo', 'Kolory dopasowane do tła', 'As cores seguem o fundo', 'Кольори під фон'),
    'panels get a hint of your wallpaper’s color, and a very dark wallpaper makes them a little lighter – your own app colors always win': (
        'panely dostanú nádych farby tvojej tapety a pri veľmi tmavej tapete sú trochu svetlejšie – tvoje vlastné farby majú vždy prednosť',
        'Bereiche bekommen einen Hauch der Farbe deines Hintergrunds, bei sehr dunklem Hintergrund werden sie etwas heller – deine eigenen Farben haben immer Vorrang',
        'los paneles toman un toque del color de tu fondo y, con un fondo muy oscuro, se aclaran un poco; tus propios colores siempre tienen prioridad',
        'les panneaux prennent une touche de la couleur de ton fond et s’éclaircissent un peu avec un fond très sombre – tes propres couleurs gagnent toujours',
        'i pannelli prendono un tocco del colore dello sfondo e con uno sfondo molto scuro diventano un po’ più chiari – i tuoi colori hanno sempre la precedenza',
        'panele dostają odcień koloru tapety, a przy bardzo ciemnej tapecie stają się nieco jaśniejsze – twoje własne kolory zawsze wygrywają',
        'os painéis ganham um toque da cor do seu papel de parede e, com um fundo muito escuro, ficam um pouco mais claros – suas cores próprias sempre têm prioridade',
        'панелі отримують відтінок кольору шпалер, а при дуже темних шпалерах стають трохи світлішими – твої власні кольори завжди мають перевагу'),
}
for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk']):
    V19[code].update({k: v[i] for k, v in _V19H.items()})

# 1.4.33 – rozpis pamäte.
_V19I = {
    'app core': ('jadro aplikácie', 'App-Kern', 'núcleo de la app', 'cœur de l’app', 'nucleo dell’app', 'rdzeń aplikacji', 'núcleo do app', 'ядро застосунку'),
    'graphics': ('grafika', 'Grafik', 'gráficos', 'graphismes', 'grafica', 'grafika', 'gráficos', 'графіка'),
    'network': ('sieť', 'Netzwerk', 'red', 'réseau', 'rete', 'sieć', 'rede', 'мережа'),
    'window': ('okno', 'Fenster', 'ventana', 'fenêtre', 'finestra', 'okno', 'janela', 'вікно'),
}
for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk']):
    V19[code].update({k: v[i] for k, v in _V19I.items()})

# 1.4.36 – Acrylic od Windows ako predvolené.
_V19J = {
    'Automatic (recommended)': ('Automaticky (odporúčané)', 'Automatisch (empfohlen)', 'Automático (recomendado)', 'Automatique (recommandé)', 'Automatico (consigliato)', 'Automatycznie (zalecane)', 'Automático (recomendado)', 'Автоматично (рекомендовано)'),
    'Wallpaper drawn by Flux': ('Tapeta kreslená Fluxom', 'Von Flux gezeichnetes Hintergrundbild', 'Fondo dibujado por Flux', 'Fond d’écran dessiné par Flux', 'Sfondo disegnato da Flux', 'Tapeta rysowana przez Flux', 'Papel de parede desenhado pelo Flux', 'Шпалери, намальовані Flux'),
    'Automatic uses Windows Acrylic: you see what is really behind the window (also a live wallpaper), and it stays see-through when you click another app. With your own background, Flux draws it.': (
        'Automaticky používa Acrylic od Windows: vidíš, čo je naozaj za oknom (aj živú tapetu), a ostane priesvitné, aj keď klikneš do iného programu. Vlastné pozadie kreslí Flux.',
        'Automatisch nutzt Windows Acrylic: Du siehst, was wirklich hinter dem Fenster ist (auch ein Live-Hintergrund), und es bleibt durchscheinend, wenn du in eine andere App klickst. Einen eigenen Hintergrund zeichnet Flux.',
        'Automático usa Acrylic de Windows: ves lo que hay realmente detrás de la ventana (también un fondo animado), y sigue translúcida cuando haces clic en otra app. Un fondo propio lo dibuja Flux.',
        'Automatique utilise Acrylic de Windows : tu vois ce qui est vraiment derrière la fenêtre (même un fond animé), et elle reste translucide quand tu cliques dans une autre app. Un fond personnalisé est dessiné par Flux.',
        'Automatico usa Acrylic di Windows: vedi ciò che c’è davvero dietro la finestra (anche uno sfondo animato), e resta traslucida quando clicchi in un’altra app. Uno sfondo personale lo disegna Flux.',
        'Automatycznie używa Acrylic z Windows: widzisz, co naprawdę jest za oknem (także animowaną tapetę), a okno zostaje przezroczyste, gdy klikniesz inną aplikację. Własne tło rysuje Flux.',
        'Automático usa o Acrylic do Windows: você vê o que está realmente atrás da janela (até um papel de parede animado), e ela continua translúcida quando você clica em outro app. Um fundo próprio é desenhado pelo Flux.',
        'Автоматично використовує Acrylic від Windows: ти бачиш, що насправді за вікном (навіть живі шпалери), і воно лишається прозорим, коли клацаєш в іншу програму. Власне тло малює Flux.'),
}
for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk']):
    V19[code].update({k: v[i] for k, v in _V19J.items()})
