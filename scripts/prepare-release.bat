@echo off
setlocal enabledelayedexpansion

echo 🚀 Dig Wallet Rust Release Preparation
echo =======================================

if "%1"=="" (
    echo Error: Please provide a version number
    echo Usage: %0 ^<version^> ^(e.g., %0 0.1.1^)
    exit /b 1
)

set VERSION=%1
echo 📦 Preparing release for version: %VERSION%

echo 🔍 Running pre-release checks...

echo   → Checking code formatting...
cargo fmt --all -- --check
if errorlevel 1 (
    echo Error: Code formatting issues found. Run 'cargo fmt' to fix.
    exit /b 1
)

echo   → Running clippy...
cargo clippy --all-targets --all-features -- -D warnings
if errorlevel 1 (
    echo Error: Clippy warnings found. Please fix them.
    exit /b 1
)

echo   → Running test suite...
cargo test --all-features -- --test-threads=1
if errorlevel 1 (
    echo Error: Tests failed. Please fix them.
    exit /b 1
)

echo   → Building documentation...
cargo doc --no-deps --all-features
if errorlevel 1 (
    echo Error: Documentation build failed.
    exit /b 1
)

echo 🔨 Building release package...
cargo build --release
if errorlevel 1 (
    echo Error: Release build failed.
    exit /b 1
)

echo 📦 Creating package...
cargo package --allow-dirty
if errorlevel 1 (
    echo Error: Package creation failed.
    exit /b 1
)

echo ✅ All pre-release checks passed!
echo.
echo 📋 Next Steps:
echo 1. Review the changes: git diff
echo 2. Update version in Cargo.toml to %VERSION%
echo 3. Commit the version change: git add . ^&^& git commit -m "Release v%VERSION%"
echo 4. Create and push tag: git tag v%VERSION% ^&^& git push origin v%VERSION%
echo 5. The GitHub workflow will automatically:
echo    - Run tests
echo    - Publish to crates.io
echo    - Create GitHub release
echo.
echo ⚠️  Make sure you have set the CRATES_IO_TOKEN secret in GitHub repository settings
echo.
echo 🎉 Release v%VERSION% is ready!
