set -e
set -x

PGO_DATA_DIR=/tmp/pgo-data
ARCH=x86_64-unknown-linux-gnu
LLVM_PROFDATA_CMD=$HOME/.rustup/toolchains/stable-$ARCH/lib/rustlib/$ARCH/bin/llvm-profdata

# STEP 0: Make sure there is no left-over profiling data from previous runs
rm -rf $PGO_DATA_DIR

# STEP 1: Build the instrumented binaries
RUSTFLAGS="-Cprofile-generate=$PGO_DATA_DIR" \
    cargo build --release --target=$ARCH

# STEP 2: Run the instrumented binaries with some typical data
./target/$ARCH/release/radbot --random
./target/$ARCH/release/radbot --random
./target/$ARCH/release/radbot --random
./target/$ARCH/release/radbot --random
./target/$ARCH/release/radbot --random
./target/$ARCH/release/radbot --random

# STEP 3: Merge the `.profraw` files into a `.profdata` file
$LLVM_PROFDATA_CMD merge -o $PGO_DATA_DIR/merged.profdata $PGO_DATA_DIR

# STEP 4: Use the `.profdata` file for guiding optimizations
RUSTFLAGS="-Cprofile-use=$PGO_DATA_DIR/merged.profdata -Cllvm-args=-pgo-warn-missing-function" \
    cargo build --release --target=$ARCH
