; The NSIS installer's hooks (bundle.windows.nsis.installerHooks in tauri.windows.conf.json).
;
; Lamina PDF registers as able to open PDFs and never as their default (ADR 0008 §10). Tauri's own
; association writes its class as `.pdf`'s default value, so tauri.windows.conf.json gives Windows
; no `fileAssociations` and these hooks register instead: a class with an open command, listed in
; `.pdf`'s OpenWithProgids, which puts the app in "Open with" and the Default apps settings while
; the user's choice stays theirs. The per-user install (installMode currentUser) writes under
; HKCU; SHCTX is the install's own hive. The MSI's twin is windows/registry.wxs.

!define LAMINA_PROGID "LaminaPDF.Document"

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr SHCTX "Software\Classes\${LAMINA_PROGID}" "" "PDF Document"
  WriteRegStr SHCTX "Software\Classes\${LAMINA_PROGID}\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
  WriteRegStr SHCTX "Software\Classes\${LAMINA_PROGID}\shell\open" "" "Open with ${PRODUCTNAME}"
  WriteRegStr SHCTX "Software\Classes\${LAMINA_PROGID}\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
  WriteRegStr SHCTX "Software\Classes\.pdf\OpenWithProgids" "${LAMINA_PROGID}" ""
  ; Tells Explorer the registrations changed (SHCNE_ASSOCCHANGED, SHCNF_FLUSH).
  System::Call "shell32::SHChangeNotify(i 0x08000000, i 0x1000, p 0, p 0)"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue SHCTX "Software\Classes\.pdf\OpenWithProgids" "${LAMINA_PROGID}"
  DeleteRegKey SHCTX "Software\Classes\${LAMINA_PROGID}"
  System::Call "shell32::SHChangeNotify(i 0x08000000, i 0x1000, p 0, p 0)"
!macroend
