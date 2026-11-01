<#
.SYNOPSIS
  Publica uma nova versão do Weft Launcher com um comando só.

.DESCRIPTION
  1. Confere o Git (branch, tag repetida, commits novos no GitHub)
  2. Sobe a versão em package.json, package-lock.json, Cargo.toml e tauri.conf.json
  3. Confere o código (npm run build e cargo check)
  4. Faz o commit, cria a tag vX.Y.Z e envia tudo ao GitHub
  5. O GitHub compila Windows, Linux e macOS sozinho
  6. Com -Publish, espera a build terminar e publica o rascunho (precisa do GitHub CLI)

.EXAMPLE
  .\scripts\release.ps1                   # 1.0.0 -> 1.0.1
  .\scripts\release.ps1 -Bump minor       # 1.0.1 -> 1.1.0
  .\scripts\release.ps1 -Version 2.0.0
  .\scripts\release.ps1 -DryRun           # só mostra o que faria
  .\scripts\release.ps1 -Publish -Notes "Corrige os selos NEW e LATEST"
#>
[CmdletBinding()]
param(
    [string]$Version,
    [ValidateSet('patch', 'minor', 'major')][string]$Bump = 'patch',
    [string]$Message,
    [string]$Notes,
    [switch]$Publish,
    [switch]$SkipChecks,
    [switch]$Yes,
    [switch]$DryRun,
    [switch]$NoOpen
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$utf8 = New-Object System.Text.UTF8Encoding($false)

function Say([string]$t)  { Write-Host ""; Write-Host $t -ForegroundColor Cyan }
function Ok([string]$t)   { Write-Host "  OK  $t" -ForegroundColor Green }
function Warn([string]$t) { Write-Host "  !!  $t" -ForegroundColor Yellow }
function P([string]$rel)  { return (Join-Path $root $rel) }

function Invoke-Git {
    & git @args
    if ($LASTEXITCODE -ne 0) { throw "git $($args -join ' ') falhou (código $LASTEXITCODE)." }
}
function Get-GitOutput {
    $out = & git @args
    if ($LASTEXITCODE -ne 0) { throw "git $($args -join ' ') falhou (código $LASTEXITCODE)." }
    return $out
}
function Invoke-Native([string]$label, [scriptblock]$cmd) {
    & $cmd
    if ($LASTEXITCODE -ne 0) { throw "$label falhou (código $LASTEXITCODE)." }
}

$fPackage = 'package.json'
$fCargo   = 'src-tauri\Cargo.toml'
$fTauri   = 'src-tauri\tauri.conf.json'

foreach ($f in @($fPackage, $fCargo, $fTauri)) {
    if (-not (Test-Path (P $f))) { throw "Arquivo não encontrado: $f. O script precisa ficar na pasta scripts\ do projeto." }
}
if (-not (Get-Command git -ErrorAction SilentlyContinue)) { throw 'O Git não está instalado ou não está no PATH.' }

function Get-JsonVersion([string]$rel) {
    $t = [IO.File]::ReadAllText((P $rel))
    if ($t -match '"version"\s*:\s*"(\d+\.\d+\.\d+)"') { return $Matches[1] }
    throw "Não achei a versão em $rel."
}
function Get-CargoVersion([string]$rel) {
    $t = [IO.File]::ReadAllText((P $rel))
    if ($t -match '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"') { return $Matches[1] }
    throw "Não achei a versão em $rel."
}

# ---------- 1. Descobrir a versão nova ----------
Say 'Conferindo o projeto'
$current = Get-JsonVersion $fTauri
$vPackage = Get-JsonVersion $fPackage
$vCargo = Get-CargoVersion $fCargo
if (($vPackage -ne $current) -or ($vCargo -ne $current)) {
    Warn "As versões estavam diferentes (tauri.conf.json $current, package.json $vPackage, Cargo.toml $vCargo). Vou deixar as três iguais."
}
$cur = [version]$current

if ($Version) {
    if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw 'Versão inválida. Use o formato 1.2.3.' }
    $new = $Version
}
else {
    switch ($Bump) {
        'major' { $new = '{0}.0.0' -f ($cur.Major + 1) }
        'minor' { $new = '{0}.{1}.0' -f $cur.Major, ($cur.Minor + 1) }
        default { $new = '{0}.{1}.{2}' -f $cur.Major, $cur.Minor, ($cur.Build + 1) }
    }
}
if ([version]$new -le $cur) { throw "A versão nova ($new) precisa ser maior que a atual ($current)." }
$tag = "v$new"

# ---------- 2. Conferir o Git ----------
$branch = (Get-GitOutput rev-parse --abbrev-ref HEAD | Select-Object -First 1).Trim()
if ($branch -eq 'HEAD') { throw 'O Git está em "detached HEAD". Volte para a branch principal (git checkout master).' }

Invoke-Git fetch origin --tags --quiet

& git rev-parse --verify --quiet "origin/$branch" | Out-Null
if ($LASTEXITCODE -eq 0) {
    $behind = [int](Get-GitOutput rev-list --count "HEAD..origin/$branch" | Select-Object -First 1)
    if ($behind -gt 0) { throw "O GitHub tem $behind commit(s) que você ainda não tem. Rode 'git pull' primeiro e tente de novo." }
}
if (Get-GitOutput tag -l $tag) { throw "A tag $tag já existe no seu computador." }
if (Get-GitOutput ls-remote --tags origin "refs/tags/$tag") { throw "A tag $tag já existe no GitHub." }

$status = Get-GitOutput status --short
if ($status) {
    Warn 'Estas alterações vão entrar na release:'
    $status | ForEach-Object { Write-Host "       $_" }
}

Say "Vai publicar: $current -> $new  (tag $tag, branch $branch)"
if ($DryRun) { Warn 'Modo de teste (-DryRun): nada foi alterado.'; return }
if (-not $Yes) {
    $ans = Read-Host 'Continuar? (S/n)'
    if ($ans -match '^(n|no|nao|não)$') { Write-Host 'Cancelado.'; return }
}

# ---------- 3. Backup, versão nova e conferência ----------
$backup = @{}
foreach ($rel in @('package.json', 'package-lock.json', 'src-tauri\Cargo.toml', 'src-tauri\Cargo.lock', 'src-tauri\tauri.conf.json')) {
    $p = P $rel
    if (Test-Path $p) { $backup[$p] = [IO.File]::ReadAllBytes($p) }
}
function Restore-Files {
    foreach ($k in $backup.Keys) { [IO.File]::WriteAllBytes($k, $backup[$k]) }
}

try {
    Say "Atualizando a versão para $new"
    Invoke-Native 'npm version' { npm version $new --no-git-tag-version --allow-same-version | Out-Null }

    $p = P $fTauri
    $t = [IO.File]::ReadAllText($p)
    $rx = New-Object System.Text.RegularExpressions.Regex('("version"\s*:\s*")\d+\.\d+\.\d+(")')
    [IO.File]::WriteAllText($p, $rx.Replace($t, ('${1}' + $new + '${2}'), 1), $utf8)

    $p = P $fCargo
    $t = [IO.File]::ReadAllText($p)
    $rx = New-Object System.Text.RegularExpressions.Regex('(?m)^(version\s*=\s*")\d+\.\d+\.\d+(")')
    [IO.File]::WriteAllText($p, $rx.Replace($t, ('${1}' + $new + '${2}'), 1), $utf8)

    foreach ($pair in @(@($fPackage, 'json'), @($fTauri, 'json'), @($fCargo, 'cargo'))) {
        if ($pair[1] -eq 'json') { $v = Get-JsonVersion $pair[0] } else { $v = Get-CargoVersion $pair[0] }
        if ($v -ne $new) { throw "Não consegui atualizar a versão em $($pair[0])." }
    }
    Ok "package.json, Cargo.toml e tauri.conf.json em $new"

    if (-not $SkipChecks) {
        Say 'Conferindo o código (pode levar alguns minutos)'
        Invoke-Native 'npm run build' { npm run build }
        Invoke-Native 'cargo check' { cargo check --manifest-path src-tauri\Cargo.toml }
        Ok 'O código compila'
    }
    else {
        Warn 'Conferência do código pulada (-SkipChecks).'
    }
}
catch {
    Restore-Files
    Write-Host ''
    Write-Host "ERRO: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host 'Os arquivos de versão voltaram ao que eram. Nada foi enviado ao GitHub.' -ForegroundColor Yellow
    exit 1
}

# ---------- 4. Commit, tag e envio ----------
try {
    Say 'Salvando no Git e enviando ao GitHub'
    if ($Message) { $msg = $Message } else { $msg = "Release $tag" }
    Invoke-Git add -A
    Invoke-Git commit -m $msg
    Invoke-Git tag $tag
    Invoke-Git push origin $branch
    Invoke-Git push origin $tag
    Ok "Enviado: $tag"
}
catch {
    Write-Host ''
    Write-Host "ERRO: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host 'O commit e a tag já existem no seu computador, mas o envio não terminou.' -ForegroundColor Yellow
    Write-Host 'Resolva o problema (internet, login) e rode:' -ForegroundColor Yellow
    Write-Host "    git push origin $branch"
    Write-Host "    git push origin $tag"
    exit 1
}

# ---------- 5. Depois do envio ----------
# a partir daqui os avisos do gh (que saem em stderr) nao devem parar o script
$ErrorActionPreference = 'Continue'
$remote = ((Get-GitOutput remote get-url origin | Select-Object -First 1)).Trim()
$web = ($remote -replace '\.git$', '') -replace '^git@github\.com:', 'https://github.com/'
if (-not $NoOpen) { Start-Process "$web/actions" }

if ($Publish) {
    if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
        Warn "O GitHub CLI (gh) não está instalado. Publique o rascunho em $web/releases."
    }
    else {
        & gh auth status 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) {
            Warn "Rode 'gh auth login' uma vez. Por enquanto, publique o rascunho em $web/releases."
        }
        else {
            Say 'Esperando a build começar'
            $runId = $null
            for ($i = 0; ($i -lt 24) -and (-not $runId); $i++) {
                Start-Sleep -Seconds 5
                $json = & gh run list --workflow release.yml --branch $tag --limit 1 --json databaseId 2>$null
                if (($LASTEXITCODE -eq 0) -and $json) {
                    $arr = @(($json -join "`n") | ConvertFrom-Json)
                    if ($arr.Count -gt 0) { $runId = $arr[0].databaseId }
                }
            }
            if (-not $runId) {
                Warn "Não achei a build. Acompanhe em $web/actions e publique o rascunho em $web/releases."
            }
            else {
                Say 'Compilando Windows, Linux e macOS (15 a 25 minutos). Pode deixar esta janela aberta.'
                & gh run watch $runId --exit-status --interval 30
                if ($LASTEXITCODE -ne 0) {
                    Warn "A build falhou. Veja os logs em $web/actions. O rascunho NÃO foi publicado."
                    exit 1
                }
                if ($Notes) { & gh release edit $tag --draft=false --latest --notes $Notes }
                else { & gh release edit $tag --draft=false --latest }
                if ($LASTEXITCODE -ne 0) {
                    Warn "Não consegui publicar sozinho. Publique em $web/releases (botão Publish release)."
                    exit 1
                }
                Ok "Release $tag publicada!"
                Write-Host "  $web/releases/tag/$tag"
            }
        }
    }
}
else {
    Write-Host ''
    Write-Host 'Próximos passos:' -ForegroundColor Cyan
    Write-Host "  1. Espere os 3 trabalhos ficarem verdes em $web/actions (15 a 25 minutos)."
    Write-Host "  2. Abra $web/releases, confira o rascunho e clique em Publish release."
    Write-Host "  3. Abra o launcher instalado: o botão de atualização deve aparecer."
    Write-Host '  (Da próxima vez, use -Publish para fazer os passos 1 e 2 sozinho.)'
}
