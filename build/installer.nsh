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

; Posledná stránka: vlastná, aby bol text „Run Flux“ pri zaškrtávacom políčku biely (Windows ho inak kreslí čiernou).
!macro customFinishPage
  !ifndef HIDE_RUN_AFTER_FINISH
    Function StartApp
      ${if} ${isUpdated}
        StrCpy $1 "--updated"
      ${else}
        StrCpy $1 ""
      ${endif}
      ${StdUtils.ExecShellAsUser} $0 "$launchLink" "open" "$1"
    FunctionEnd

    Function FluxFinishShow
      ; bez témy Windows sa dá nastaviť farba textu políčka
      System::Call 'UxTheme::SetWindowTheme(p $mui.FinishPage.Run, w " ", w " ")'
      SetCtlColors $mui.FinishPage.Run "FFFFFF" "101016"
    FunctionEnd

    !define MUI_FINISHPAGE_RUN
    !define MUI_FINISHPAGE_RUN_FUNCTION "StartApp"
    !define MUI_FINISHPAGE_RUN_TEXT "Start Flux now"
    !define MUI_PAGE_CUSTOMFUNCTION_SHOW FluxFinishShow
  !endif
  !define MUI_FINISHPAGE_TITLE "Flux is ready"
  !define MUI_FINISHPAGE_TEXT "Flux has been installed on your computer. It keeps itself up to date – new versions install when you close it."
  !insertmacro MUI_PAGE_FINISH
!macroend
