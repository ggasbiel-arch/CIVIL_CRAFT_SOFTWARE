@echo off
cd /d "%~dp0"
where cargo >nul 2>&1
if errorlevel 1 (
  echo Rust/Cargo nao encontrado. Instale via https://rustup.rs/
  pause
  exit /b 1
)
cargo run --release -p civil_desktop
if errorlevel 1 (
  echo O programa nao compilou ou ocorreu um erro. Verifique as mensagens acima.
)
pause
