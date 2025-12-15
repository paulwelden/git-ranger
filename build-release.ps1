#!/usr/bin/env pwsh
# Git Ranger - Release Build Script
# This script builds the application in release mode for optimal performance

param(
    [switch]$Clean,
    [switch]$Install,
    [switch]$Help
)

function Show-Help {
    Write-Host @"
Git Ranger Release Build Script

Usage: .\build-release.ps1 [-Clean] [-Install] [-Help]

Options:
  -Clean     Clean build artifacts before building
  -Install   Install the binary to cargo's bin directory after building
  -Help      Show this help message

Examples:
  .\build-release.ps1                  # Build in release mode
  .\build-release.ps1 -Clean           # Clean and build
  .\build-release.ps1 -Clean -Install  # Clean, build, and install to PATH

Output:
  The compiled binary will be located at: target\release\git-ranger.exe
"@
}

if ($Help) {
    Show-Help
    exit 0
}

Write-Host "🔨 Building Git Ranger in release mode..." -ForegroundColor Cyan

if ($Clean) {
    Write-Host "🧹 Cleaning build artifacts..." -ForegroundColor Yellow
    cargo clean
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Clean failed!" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

Write-Host "📦 Compiling with optimizations..." -ForegroundColor Yellow
cargo build --release

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "✅ Build successful!" -ForegroundColor Green
Write-Host "📍 Binary location: $PWD\target\release\git-ranger.exe" -ForegroundColor Cyan

if ($Install) {
    Write-Host "📥 Installing to cargo bin directory..." -ForegroundColor Yellow
    cargo install --path .
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Installation successful!" -ForegroundColor Green
        Write-Host "🎯 You can now run 'git-ranger' from anywhere" -ForegroundColor Cyan
    } else {
        Write-Host "❌ Installation failed!" -ForegroundColor Red
        exit $LASTEXITCODE
    }
} else {
    Write-Host ""
    Write-Host "💡 Tip: Run '.\build-release.ps1 -Install' to install to your PATH" -ForegroundColor Yellow
    Write-Host "💡 Or manually run: .\target\release\git-ranger.exe" -ForegroundColor Yellow
}
