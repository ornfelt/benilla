# Recompiles every `.gfxs` source under `src/` into the four shader families the loader picks
# from at runtime (`crates/benilla-gfx/src/shader_loader.rs`); the Windows twin of `compile.sh`:
#   shaders/            gl3, gl4, gles3
#   shaders_vk/         vk (SPIR-V, through glslangValidator)
#   shaders_d3/         d3d11 (HLSL; d3d12 reads the same blob)
#   shaders_gles3_dark/ gles3 (the native-window gles3 family off Linux)
#
# The compiler is `wc_compiler_rs`: `$env:GFX_SHADER_COMPILER`, else the release or debug build
# under `$env:code_root_dir\Code2\General\gfx\wc_compiler_rs` (`cargo build --release` there);
# `code_root_dir` defaults to the user profile.
#
#   crates\benilla-gfx\shaders\compile.ps1          # all
#   crates\benilla-gfx\shaders\compile.ps1 blit     # one base name
param([string[]]$Names)

$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$root = if ($env:code_root_dir) { $env:code_root_dir } else { $env:USERPROFILE }

$compiler = $env:GFX_SHADER_COMPILER
if (-not $compiler) {
    foreach ($profile in 'release', 'debug') {
        $c = Join-Path $root "Code2\General\gfx\wc_compiler_rs\target\$profile\wc_compiler_rs.exe"
        if (Test-Path -LiteralPath $c -PathType Leaf) { $compiler = $c; break }
    }
}
if (-not $compiler -or -not (Test-Path -LiteralPath $compiler -PathType Leaf)) {
    Write-Error 'compile.ps1: wc_compiler_rs not found; set GFX_SHADER_COMPILER'
    exit 1
}

$families = @(
    @{ Dir = 'shaders';            Backends = 'gl3,gl4,gles3' },
    @{ Dir = 'shaders_vk';         Backends = 'vk' },
    @{ Dir = 'shaders_d3';         Backends = 'd3d11' },
    @{ Dir = 'shaders_gles3_dark'; Backends = 'gles3' }
)

if (-not $Names -or $Names.Count -eq 0) {
    $Names = Get-ChildItem -LiteralPath (Join-Path $here 'src') -Filter '*.vs.gfxs' |
        Sort-Object Name |
        ForEach-Object { $_.Name -replace '\.vs\.gfxs$', '' }
}

$failed = $false
foreach ($family in $families) {
    $dir = Join-Path $here $family.Dir
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    foreach ($name in $Names) {
        foreach ($stage in 'vs', 'fs') {
            $src = Join-Path $here "src\$name.$stage.gfxs"
            if (-not (Test-Path -LiteralPath $src -PathType Leaf)) {
                Write-Host "compile.ps1: missing $src" -ForegroundColor Red
                $failed = $true
                continue
            }
            $out = Join-Path $dir "$name.$stage.gfx"
            & $compiler -t $stage -x $family.Backends -i $src -o $out | Out-Null
            if ($LASTEXITCODE -ne 0) {
                Write-Host "compile.ps1: FAILED $name.$stage ($($family.Backends))" -ForegroundColor Red
                $failed = $true
            }
        }
    }
    Write-Host "compile.ps1: $($family.Dir) ($($family.Backends)): $($Names.Count) shader(s)"
}
if ($failed) { exit 1 }
exit 0
