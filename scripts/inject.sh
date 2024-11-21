#!/usr/bin/env bash
set -eo pipefail

iso="$1"
if [ ! -f "$iso" ]; then
  echo "Usage: $0 <iso> <key>"
  exit 1
fi

key="$2"
if [ ! -f "$key" ]; then
  echo "Usage: $0 <iso> <key>"
  exit 1
fi

temp="$(mktemp -d)"
loop="$(losetup -f)"

losetup -P "$loop" "$iso"
mount "${loop}p2" "$temp"

mkdir -p "$temp/root"
chown root:root "$temp/root"
chmod 700 "$temp/root/"
cp -f "$key" "$temp/root/secrets.age"
chown root:root "$temp/root/secrets.age"
chmod 400 "$temp/root/secrets.age"

sudo umount "$temp"
losetup -d "$loop"
