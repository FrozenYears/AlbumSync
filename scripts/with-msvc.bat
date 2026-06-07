@echo off
REM with-msvc.bat
REM Run any command inside a vcvars64 (MSVC + Windows SDK) shell.
REM
REM Why this exists:
REM   - git bash has /usr/bin/link (MSYS2 coreutils) which hijacks rustc's
REM     attempt to call link.exe; without LIB/INCLUDE set, kernel32.lib
REM     etc. cannot be located.
REM   - vcvars64.bat itself needs vswhere.exe on PATH to locate the
REM     Windows SDK headers/libs; without it, only the MSVC toolset is
REM     wired up and Windows SDK libs (kernel32.lib, etc.) go missing.
REM   - Calling cargo / pnpm-tauri through this wrapper guarantees the
REM     correct MSVC + Windows SDK environment.
REM
REM Usage from git bash:
REM   cmd.exe //c scripts/with-msvc.bat cargo check
REM   cmd.exe //c scripts/with-msvc.bat cargo build --release
REM   cmd.exe //c scripts/with-msvc.bat pnpm tauri dev

set "VSWHERE_DIR=C:\Program Files (x86)\Microsoft Visual Studio\Installer"
set "VS_VCVARS=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if not exist "%VS_VCVARS%" (
  echo [with-msvc] ERROR: cannot find %VS_VCVARS% 1>&2
  echo [with-msvc] Edit scripts/with-msvc.bat to point at your VS install. 1>&2
  exit /b 2
)
set "PATH=%VSWHERE_DIR%;%USERPROFILE%\.cargo\bin;%PATH%"
call "%VS_VCVARS%" >NUL
%*
