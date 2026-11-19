#!/bin/bash
# WSLでreleaseビルド→VPSへ転送→再起動(旧バイナリは .prev に退避)
set -e
cd "$(dirname "$0")"
MSYS_NO_PATHCONV=1 wsl -d Ubuntu -- bash /mnt/f/runo/repository/aruaru.pro/.build-wsl.sh | tail -1
MSYS_NO_PATHCONV=1 wsl -d Ubuntu -- bash -c 'cp $HOME/aruaru-pro-target/release/aruaru-pro /mnt/f/runo/repository/aruaru.pro/target/aruaru-pro.bin'
scp -q target/aruaru-pro.bin conoha:/root/aruaru.pro/aruaru-pro.new
ssh conoha 'cd /root/aruaru.pro && cp -f aruaru-pro aruaru-pro.prev 2>/dev/null; mv aruaru-pro.new aruaru-pro && chmod +x aruaru-pro && systemctl restart aruaru-pro; sleep 10; systemctl is-active aruaru-pro'
