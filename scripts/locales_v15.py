# Flux 1.4: plynulé posúvanie všade, prechodové animácie, priebeh aktualizácie (sk, de, es, fr, it, pl, pt, uk).
_V15 = {
    'Installing… {p} %': ('Inštalujem… {p} %', 'Installation … {p} %', 'Instalando… {p} %', 'Installation… {p} %', 'Installazione… {p} %', 'Instalowanie… {p} %', 'Instalando… {p} %', 'Встановлення… {p} %'),
    'Preparing…': ('Pripravujem…', 'Wird vorbereitet …', 'Preparando…', 'Préparation…', 'Preparazione…', 'Przygotowywanie…', 'Preparando…', 'Підготовка…'),
    'Restart to update': ('Reštartuj a aktualizuj', 'Neu starten zum Aktualisieren', 'Reinicia para actualizar', 'Redémarrer pour mettre à jour', 'Riavvia per aggiornare', 'Uruchom ponownie, aby zaktualizować', 'Reinicie para atualizar', 'Перезапусти для оновлення'),
    'Updating {p} %': ('Aktualizujem {p} %', 'Aktualisierung {p} %', 'Actualizando {p} %', 'Mise à jour {p} %', 'Aggiornamento {p} %', 'Aktualizacja {p} %', 'Atualizando {p} %', 'Оновлення {p} %'),
    'Transition animations': ('Prechodové animácie', 'Übergangsanimationen', 'Animaciones de transición', 'Animations de transition', 'Animazioni di transizione', 'Animacje przejść', 'Animações de transição', 'Анімації переходів'),
    'a soft fade when you switch files, settings pages and screens': (
        'jemné prelínanie pri prepnutí súboru, stránky nastavení a obrazovky', 'sanftes Einblenden beim Wechsel von Dateien, Einstellungsseiten und Ansichten',
        'un fundido suave al cambiar de archivo, página de ajustes o pantalla', 'un fondu doux quand tu changes de fichier, de page de réglages ou d’écran',
        'una dissolvenza morbida quando cambi file, pagina delle impostazioni o schermata', 'łagodne przejście przy zmianie pliku, strony ustawień i ekranu',
        'um esmaecer suave ao trocar de arquivo, página de configurações e tela', 'плавна поява під час перемикання файлів, сторінок налаштувань і екранів'),
    'the editor, settings, lists and panels keep gliding a bit after you stop the wheel': (
        'editor, nastavenia, zoznamy a panely sa po pustení kolieska ešte chvíľu posúvajú', 'Editor, Einstellungen, Listen und Bereiche gleiten nach dem Mausrad noch kurz weiter',
        'el editor, los ajustes, las listas y los paneles siguen deslizándose un poco tras soltar la rueda', 'l’éditeur, les réglages, les listes et les panneaux glissent encore un peu après la molette',
        'editor, impostazioni, elenchi e pannelli scorrono ancora un po’ dopo la rotella', 'edytor, ustawienia, listy i panele przesuwają się jeszcze chwilę po puszczeniu kółka',
        'o editor, as configurações, as listas e os painéis deslizam um pouco depois da roda', 'редактор, налаштування, списки й панелі ще трохи ковзають після коліщатка'),
}
V15 = {code: {k: v[i] for k, v in _V15.items()} for i, code in enumerate(['sk', 'de', 'es', 'fr', 'it', 'pl', 'pt', 'uk'])}
