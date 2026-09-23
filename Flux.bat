@echo off
rem Spustenie Flux z priecinka s kodom (dvojklik). Pri prvom spusteni nainstaluje zavislosti.
cd /d "%~dp0"
where npm >nul 2>nul
if errorlevel 1 (
  echo Chyba: nie je nainstalovany Node.js. Stiahni ho z https://nodejs.org alebo spusti:
  echo     winget install OpenJS.NodeJS.LTS
  pause
  exit /b 1
)
if not exist node_modules (
  echo Instalujem zavislosti, prvykrat to chvilu potrva...
  call npm install || (pause & exit /b 1)
)
call npm start
