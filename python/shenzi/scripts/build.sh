#! /bin/bash
set -euxo pipefail

cleanup() {
  rm src/shenzi/bin/shenzi || true
}
trap cleanup EXIT # This will run this script on exit, even if it exits with error midway

# run from python/shenzi (root of the python package)
rm -rf dist || true

(
    cd ../../crates/shenzi
    cargo build --release
)


mkdir src/shenzi/bin || true
touch src/shenzi/bin/__init__.py
cp ../../crates/shenzi/target/release/shenzi ./src/shenzi/bin/
uv build