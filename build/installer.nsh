; Vzhľad inštalátora Fluxu: tmavé uvítanie a koniec s obrázkom (build/installerSidebar.bmp),
; tmavá hlavička s logom. Obrázky vytvorí: npx electron scripts/make-installer-art.cjs

!macro customWelcomePage
  !define MUI_BGCOLOR "101016"
  !define MUI_TEXTCOLOR "FFFFFF"
  !define MUI_WELCOMEPAGE_TITLE "Welcome to Flux"
  !define MUI_WELCOMEPAGE_TEXT "Flux is a small, modern code editor – run your code with one click, see websites live and get help from AI.$\r$\n$\r$\nProgramming languages are not bundled: Flux downloads only the ones you pick, so this installer stays small.$\r$\n$\r$\nClick Next to continue."
  !insertmacro MUI_PAGE_WELCOME
!macroend

!macro customUnWelcomePage
  !define MUI_WELCOMEPAGE_TITLE "Uninstall Flux"
  !define MUI_WELCOMEPAGE_TEXT "Flux will be removed from this computer. Your projects and files stay where they are.$\r$\n$\r$\nClick Next to continue."
  !insertmacro MUI_UNPAGE_WELCOME
!macroend
