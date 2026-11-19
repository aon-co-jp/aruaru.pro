#!/bin/bash
export PATH=/usr/local/bin:/usr/bin:/bin:$HOME/.cargo/bin
cd /mnt/f/runo/repository/aruaru.pro
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=$HOME/aruaru-pro-target cargo build --release 2>&1 | tail -4
ls -la $HOME/aruaru-pro-target/release/aruaru-pro
