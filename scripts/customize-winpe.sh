#!/bin/bash
# customize-winpe.sh - Script to inject deployment scripts into WinPE

set -e

if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <path-to-boot.wim> [server-url]"
    echo ""
    echo "This script customizes a WinPE boot.wim with Snow-Owl deployment scripts."
    echo ""
    echo "Arguments:"
    echo "  path-to-boot.wim  Path to the WinPE boot.wim file"
    echo "  server-url        Optional: Default server URL (e.g., http://192.168.100.1:8080)"
    exit 1
fi

BOOT_WIM="$1"
SERVER_URL="${2:-http://192.168.100.1:8080}"
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
MOUNT_DIR="/tmp/winpe-mount"

echo "Snow-Owl WinPE Customization Script"
echo "===================================="
echo ""
echo "Boot WIM: $BOOT_WIM"
echo "Server URL: $SERVER_URL"
echo ""

# Check if DISM is available (on Windows) or wimlib (on Linux)
if command -v wimlib-imagex &> /dev/null; then
    WIMTOOL="wimlib-imagex"
    echo "Using wimlib-imagex"
elif command -v dism.exe &> /dev/null; then
    WIMTOOL="dism"
    echo "Using Windows DISM"
else
    echo "ERROR: Neither wimlib nor DISM found."
    echo "On Linux, install wimlib-utils: sudo apt install wimlib-tools"
    echo "On Windows, DISM should be available by default."
    exit 1
fi

# Create mount directory
mkdir -p "$MOUNT_DIR"

echo "Mounting WinPE image..."
if [ "$WIMTOOL" = "wimlib-imagex" ]; then
    wimlib-imagex mount "$BOOT_WIM" 1 "$MOUNT_DIR"
else
    dism.exe /Mount-Wim /WimFile:"$BOOT_WIM" /Index:1 /MountDir:"$MOUNT_DIR"
fi

echo "Copying deployment scripts..."

# Copy PowerShell deployment script
mkdir -p "$MOUNT_DIR/Deploy"
cp "$SCRIPT_DIR/winpe/Deploy-Windows.ps1" "$MOUNT_DIR/Deploy/"

# Copy diskpart scripts
cp "$SCRIPT_DIR/winpe/diskpart-uefi.txt" "$MOUNT_DIR/Deploy/"
cp "$SCRIPT_DIR/winpe/diskpart-bios.txt" "$MOUNT_DIR/Deploy/"

# Update startnet.cmd with server URL
sed "s|SERVER_URL=.*|SERVER_URL=$SERVER_URL|g" "$SCRIPT_DIR/winpe/startnet.cmd" > "$MOUNT_DIR/Windows/System32/startnet.cmd"

echo "Scripts copied successfully."

echo "Unmounting WinPE image..."
if [ "$WIMTOOL" = "wimlib-imagex" ]; then
    wimlib-imagex unmount "$MOUNT_DIR" --commit
else
    dism.exe /Unmount-Wim /MountDir:"$MOUNT_DIR" /Commit
fi

# Cleanup
rmdir "$MOUNT_DIR" 2>/dev/null || true

echo ""
echo "WinPE customization complete!"
echo ""
echo "Next steps:"
echo "1. Copy the modified boot.wim to your Snow-Owl winpe directory"
echo "2. Start the Snow-Owl server"
echo "3. Boot your target machines via PXE"
