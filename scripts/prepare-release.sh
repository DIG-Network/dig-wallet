#!/bin/bash

# Dig Wallet Rust Release Preparation Script
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Dig Wallet Rust Release Preparation${NC}"
echo "======================================="

# Check if version argument is provided
if [ $# -eq 0 ]; then
    echo -e "${RED}Error: Please provide a version number${NC}"
    echo "Usage: $0 <version> (e.g., $0 0.1.1)"
    exit 1
fi

VERSION=$1
echo -e "${BLUE}📦 Preparing release for version: ${VERSION}${NC}"

# Validate version format (basic check)
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Error: Version must be in format x.y.z (e.g., 0.1.1)${NC}"
    exit 1
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must be run from the project root directory${NC}"
    exit 1
fi

echo -e "${YELLOW}🔍 Running pre-release checks...${NC}"

# 1. Check formatting
echo "  → Checking code formatting..."
cargo fmt --all -- --check
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Code formatting issues found. Run 'cargo fmt' to fix.${NC}"
    exit 1
fi

# 2. Check clippy
echo "  → Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Clippy warnings found. Please fix them.${NC}"
    exit 1
fi

# 3. Run tests
echo "  → Running test suite..."
cargo test --all-features -- --test-threads=1
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Tests failed. Please fix them.${NC}"
    exit 1
fi

# 4. Check documentation
echo "  → Building documentation..."
cargo doc --no-deps --all-features
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Documentation build failed.${NC}"
    exit 1
fi

# 5. Update version in Cargo.toml
echo -e "${YELLOW}📝 Updating version in Cargo.toml...${NC}"
if command -v sed > /dev/null; then
    # Use sed to update version
    sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
    rm -f Cargo.toml.bak
else
    echo -e "${YELLOW}Warning: sed not found. Please manually update version in Cargo.toml to $VERSION${NC}"
fi

# 6. Verify package can be built
echo -e "${YELLOW}🔨 Building release package...${NC}"
cargo build --release
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Release build failed.${NC}"
    exit 1
fi

# 7. Verify package can be packaged
echo -e "${YELLOW}📦 Creating package...${NC}"
cargo package --allow-dirty
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Package creation failed.${NC}"
    exit 1
fi

echo -e "${GREEN}✅ All pre-release checks passed!${NC}"
echo ""
echo -e "${BLUE}📋 Next Steps:${NC}"
echo "1. Review the changes: git diff"
echo "2. Commit the version change: git add . && git commit -m \"Release v$VERSION\""
echo "3. Create and push tag: git tag v$VERSION && git push origin v$VERSION"
echo "4. The GitHub workflow will automatically:"
echo "   - Run tests"
echo "   - Publish to crates.io"
echo "   - Create GitHub release"
echo ""
echo -e "${YELLOW}⚠️  Make sure you have set the CRATES_IO_TOKEN secret in GitHub repository settings${NC}"
echo ""
echo -e "${GREEN}🎉 Release v$VERSION is ready!${NC}"
