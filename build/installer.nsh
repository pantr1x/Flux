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

; Posledná stránka: vlastná, aby bol text „Start Flux now“ pri zaškrtávacom políčku biely (Windows ho inak kreslí čiernou).
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

    !define MUI_FINISHPAGE_RUN
    !define MUI_FINISHPAGE_RUN_FUNCTION "StartApp"
    !define MUI_FINISHPAGE_RUN_TEXT "Start Flux now"
    !define MUI_PAGE_CUSTOMFUNCTION_SHOW FluxFinishShow
  !endif
  !define MUI_FINISHPAGE_TITLE "Flux is ready"
  !define MUI_FINISHPAGE_TEXT "Flux has been installed on your computer. It keeps itself up to date – new versions install when you close it."
  !insertmacro MUI_PAGE_FINISH

  !ifndef HIDE_RUN_AFTER_FINISH
    ; až po MUI_PAGE_FINISH existuje premenná s políčkom
    Function FluxFinishShow
      ; Políčko ostane moderné (kreslí ho Windows), text vedľa neho je samostatný biely popis.
      Push $0
      Push $1
      Push $2
      Push $3
      Push $4
      Push $5
      System::Call "*(i 0, i 0, i 0, i 0) p .r1"
      System::Call "user32::GetWindowRect(p $mui.FinishPage.Run, p r1)"
      System::Call "user32::MapWindowPoints(p 0, p $mui.FinishPage, p r1, i 2)"
      System::Call "*$1(i .r2, i .r3, i .r4, i .r5)"
      System::Free $1
      IntOp $5 $5 - $3
      SetCtlColors $mui.FinishPage.Run "FFFFFF" "101016"
      SendMessage $mui.FinishPage.Run 0x000C 0 "STR:"
      System::Call "user32::SetWindowPos(p $mui.FinishPage.Run, p 0, i r2, i r3, i 18, i r5, i 0x14)"
      IntOp $0 $2 + 24
      IntOp $4 $4 - $0
      System::Call "user32::CreateWindowEx(i 0, t 'STATIC', t 'Start Flux now', i 0x50000200, i r0, i r3, i r4, i r5, p $mui.FinishPage, p 0, p 0, p 0) p .r1"
      SendMessage $mui.FinishPage.Run 0x0031 0 0 $2
      SendMessage $1 0x0030 $2 1
      SetCtlColors $1 "FFFFFF" "101016"
      Pop $5
      Pop $4
      Pop $3
      Pop $2
      Pop $1
      Pop $0
    FunctionEnd
  !endif
!macroend
