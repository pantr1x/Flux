; Vzhľad inštalátora Fluxu: tmavé uvítanie a koniec s obrázkom (build/installerSidebar.bmp),
; tmavá hlavička s logom. Obrázky vytvorí: npx electron scripts/make-installer-art.cjs

!macro customWelcomePage
  !define MUI_BGCOLOR "101016"
  !define MUI_TEXTCOLOR "FFFFFF"
  !define MUI_WELCOMEPAGE_TITLE "Welcome to Flux"
  !define MUI_WELCOMEPAGE_TEXT "Flux is a small, modern code editor – run your code with one click, see websites live and get help from AI.$\r$\n$\r$\nProgramming languages are not bundled: Flux downloads only the ones you pick, so this installer stays small.$\r$\n$\r$\nClick Next to continue."
  ; aktualizácia z Fluxu: uvítanie sa preskočí – ukáže sa len priebeh inštalácie
  !insertmacro skipPageIfUpdated
  !insertmacro MUI_PAGE_WELCOME
!macroend

; Aktualizácia z Fluxu: nepýtať sa „pre koho inštalovať“ – rovnako ako doteraz (pre teba alebo pre všetkých).
!macro customInstallMode
  ${if} ${isUpdated}
    ${if} $hasPerUserInstallation == "1"
      StrCpy $isForceCurrentInstall "1"
    ${elseif} $hasPerMachineInstallation == "1"
      StrCpy $isForceMachineInstall "1"
    ${endif}
  ${endif}
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
  ; aktualizácia z Fluxu: bez poslednej stránky – Flux sa po inštalácii otvorí sám (customInstall)
  !define MUI_PAGE_CUSTOMFUNCTION_PRE FluxFinishPre
  Function FluxFinishPre
    ${if} ${isUpdated}
      Abort
    ${endif}
  FunctionEnd
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

; Flux v ponuke „Otvoriť v programe“ pre textové a zdrojové súbory.
; Predvolený program sa nemení – Flux sa len pridá do zoznamu.
!macro FluxTypes M
  !insertmacro ${M} txt
  !insertmacro ${M} md
  !insertmacro ${M} markdown
  !insertmacro ${M} log
  !insertmacro ${M} csv
  !insertmacro ${M} json
  !insertmacro ${M} xml
  !insertmacro ${M} yml
  !insertmacro ${M} yaml
  !insertmacro ${M} toml
  !insertmacro ${M} ini
  !insertmacro ${M} cfg
  !insertmacro ${M} py
  !insertmacro ${M} pyw
  !insertmacro ${M} js
  !insertmacro ${M} mjs
  !insertmacro ${M} cjs
  !insertmacro ${M} ts
  !insertmacro ${M} jsx
  !insertmacro ${M} tsx
  !insertmacro ${M} html
  !insertmacro ${M} htm
  !insertmacro ${M} css
  !insertmacro ${M} scss
  !insertmacro ${M} java
  !insertmacro ${M} c
  !insertmacro ${M} h
  !insertmacro ${M} cpp
  !insertmacro ${M} hpp
  !insertmacro ${M} cs
  !insertmacro ${M} go
  !insertmacro ${M} rs
  !insertmacro ${M} rb
  !insertmacro ${M} php
  !insertmacro ${M} lua
  !insertmacro ${M} sh
  !insertmacro ${M} bat
  !insertmacro ${M} ps1
  !insertmacro ${M} sql
!macroend

!macro FluxAddType EXT
  WriteRegStr SHCTX "Software\Classes\.${EXT}\OpenWithProgids" "Flux.File" ""
  WriteRegStr SHCTX "Software\Classes\Applications\${APP_EXECUTABLE_FILENAME}\SupportedTypes" ".${EXT}" ""
!macroend

!macro FluxRemoveType EXT
  DeleteRegValue SHCTX "Software\Classes\.${EXT}\OpenWithProgids" "Flux.File"
!macroend

!macro customInstall
  WriteRegStr SHCTX "Software\Classes\Flux.File" "" "Flux file"
  WriteRegStr SHCTX "Software\Classes\Flux.File\DefaultIcon" "" "$INSTDIR\${APP_EXECUTABLE_FILENAME},0"
  WriteRegStr SHCTX "Software\Classes\Flux.File\shell\open\command" "" '"$INSTDIR\${APP_EXECUTABLE_FILENAME}" "%1"'
  WriteRegStr SHCTX "Software\Classes\Applications\${APP_EXECUTABLE_FILENAME}" "FriendlyAppName" "Flux"
  WriteRegStr SHCTX "Software\Classes\Applications\${APP_EXECUTABLE_FILENAME}\DefaultIcon" "" "$INSTDIR\${APP_EXECUTABLE_FILENAME},0"
  WriteRegStr SHCTX "Software\Classes\Applications\${APP_EXECUTABLE_FILENAME}\shell\open\command" "" '"$INSTDIR\${APP_EXECUTABLE_FILENAME}" "%1"'
  !insertmacro FluxTypes FluxAddType
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
  ; Po „Reštartovať teraz“ vo Fluxe: znova ho otvoriť (poistka k --force-run).
  ; Keby sa spustil dvakrát, druhá kópia sa sama zavrie (Flux beží len raz).
  ${if} ${isUpdated}
    ReadEnvStr $R9 TEMP
    ${if} ${FileExists} "$R9\flux-relaunch-after-update"
      Delete "$R9\flux-relaunch-after-update"
      ${StdUtils.ExecShellAsUser} $R8 "$INSTDIR\${APP_EXECUTABLE_FILENAME}" "open" "--updated"
    ${endif}
  ${endif}
!macroend

!macro customUnInstall
  !insertmacro FluxTypes FluxRemoveType
  DeleteRegKey SHCTX "Software\Classes\Flux.File"
  DeleteRegKey SHCTX "Software\Classes\Applications\${APP_EXECUTABLE_FILENAME}"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend
