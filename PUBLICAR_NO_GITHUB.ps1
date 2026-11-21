#requires -Version 5.1
$ErrorActionPreference = "Stop"
Set-Location -LiteralPath $PSScriptRoot
$repo = "ggasbiel-arch/CIVIL_CRAFT_CAD_BIM"

foreach ($cmd in @("git", "gh")) {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
        throw "Necessario instalar '$cmd'. Consulte PUBLICAR_NO_GITHUB.md."
    }
}

gh auth status
if ($LASTEXITCODE -ne 0) { throw "Autentique sua conta com: gh auth login" }
$login = (gh api user --jq '.login').Trim()
if ($LASTEXITCODE -ne 0 -or $login -ne 'ggasbiel-arch') {
    throw "Conta GitHub incorreta. Esta operacao requer 'ggasbiel-arch'. Conta atual: $login"
}

# Evita publicar sobre um repo existente, alterando uma historia de modo inesperado.
$existingNames = @(gh repo list ggasbiel-arch --limit 1000 --json name --jq '.[].name')
if ($LASTEXITCODE -ne 0) { throw "Nao foi possivel verificar repositorios existentes." }
if ($existingNames -contains "CIVIL_CRAFT_CAD_BIM") {
    throw "O repositorio ja existe: $repo. Nao foi sobrescrito."
}

if (-not (Test-Path .git)) {
    git init -b main
    if ($LASTEXITCODE -ne 0) { throw "Falha ao iniciar Git" }
}
git add .
if ($LASTEXITCODE -ne 0) { throw "Falha em git add" }
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) {
    git commit -m "chore: bootstrap independent CAD/BIM and Codex instructions"
    if ($LASTEXITCODE -ne 0) { throw "Falha em git commit; configure user.name e user.email" }
}
gh repo create $repo --private --source . --remote origin --push
if ($LASTEXITCODE -ne 0) { throw "Falha ao criar/enviar repositorio. Consulte PUBLICAR_NO_GITHUB.md" }
Write-Host "Publicado: https://github.com/$repo"
