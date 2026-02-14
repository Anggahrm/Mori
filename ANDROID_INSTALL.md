# Android Installation Guide

## Download

1. Go to [GitHub Actions](https://github.com/Anggahrm/Mori/actions)
2. Select latest "Build Android" workflow
3. Download the artifact for your device:
   - `mori-android-arm64` (8 MB) - For modern Android devices
   - `mori-android-armv7` (7 MB) - For older Android devices

## Installation Methods

### Method 1: Termux (Recommended - No Root Required)

1. Install Termux from F-Droid
2. Open Termux and run:

```bash
# Update packages
pkg update && pkg upgrade

# Install wget
pkg install wget

# Download Mori (replace URL with actual download link)
wget https://github.com/Anggahrm/Mori/actions/downloads/XXX/mori-android-arm64

# Make executable
chmod +x mori-android-arm64

# Run
./mori-android-arm64
```

### Method 2: ADB (Requires USB Debugging)

```bash
# Push to device
adb push mori-android-arm64 /data/local/tmp/

# Make executable
adb shell "chmod +x /data/local/tmp/mori-android-arm64"

# Run
adb shell "/data/local/tmp/mori-android-arm64"
```

### Method 3: Root Device (Requires Root Access)

```bash
# Push to device
adb push mori-android-arm64 /data/local/tmp/

# Get root shell
adb shell
su

# Copy to system bin
cp /data/local/tmp/mori-android-arm64 /system/bin/mori

# Set permissions
chmod 755 /system/bin/mori

# Run
mori
```

## Troubleshooting

### Permission Denied
```bash
chmod +x mori-android-arm64
```

### Binary Not Found
```bash
# Use full path
./mori-android-arm64

# Or add to PATH
export PATH=$PATH:$(pwd)
```

### Network Issues
```bash
# Check internet connection
ping google.com

# Check Termux packages
pkg update
```

## Requirements

- Android 5.0 (API 21) or higher
- ARM64 or ARMv7 architecture
- Internet connection for game server communication
- 50+ MB free storage space

## Device Architecture Check

Run in Termux to check your device:
```bash
uname -m
```

Output:
- `aarch64` → Download `mori-android-arm64`
- `armv7l` → Download `mori-android-armv7`

## Notes

- This is a native Rust binary, not an APK
- Requires command-line interface (no GUI)
- Best performance on ARM64 devices
- May require Termux:API 28 or higher for some features
