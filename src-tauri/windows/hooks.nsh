; Ganchos del instalador NSIS (bundle.windows.nsis.installerHooks en tauri.conf.json).

; qapture-mcp.exe lo abren los agentes (Claude Code, Codex) y sigue corriendo mientras su sesión
; esté abierta: bloquea su propio archivo y el instalador no puede reemplazarlo. Se cierra antes
; de que el instalador cierre Qapture (CheckIfAppIsRunning). Después de actualizar, el agente
; vuelve a conectar el servidor MCP (en Claude Code, /mcp).
!macro NSIS_HOOK_PREINSTALL
  !insertmacro RestartManager_ShutdownFile "$INSTDIR\qapture-mcp.exe" $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro RestartManager_ShutdownFile "$INSTDIR\qapture-mcp.exe" $0
!macroend
