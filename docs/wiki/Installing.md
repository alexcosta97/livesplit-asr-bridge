# Installing

## Downloading

Get the latest version from [the latest release](https://github.com/alexcosta97/livesplit-asr-bridge/releases/latest). Pick the file for the PC that runs your game:

| Your PC | File |
|---|---|
| Linux | `livesplit-asr-bridge-<version>-x86_64-linux.tar.gz` |
| macOS, Apple Silicon | `livesplit-asr-bridge-<version>-arm64-macos.zip` |
| macOS, Intel | `livesplit-asr-bridge-<version>-x86_64-macos.zip` |
| Windows | `livesplit-asr-bridge-<version>-x86_64-windows.zip` |

Not sure which Mac you have? Open the Apple menu, then **About This Mac**. A chip named M1 or later is Apple Silicon. A processor named Intel is Intel.

Releases marked **Pre-release** are release candidates for testing. The latest full release is the one the link above opens.

## Opening the app for the first time

Releases aren't code-signed yet, so the first launch shows a warning.

**macOS:** unzip the download and open `livesplit-asr-bridge.app`. macOS says it can't verify the developer. Click **Done**, then go to **System Settings → Privacy & Security**, scroll down and click **Open Anyway** next to the message about livesplit-asr-bridge. After that, the app opens normally.

<!-- screenshot: T3 macos-open-anyway.png -->
The Privacy & Security page in macOS System Settings, with the Open Anyway button next to the message about livesplit-asr-bridge.

**Windows:** if SmartScreen says "Windows protected your PC", click **More info**, then **Run anyway**.

<!-- screenshot: T4 windows-smartscreen.png -->
The Windows SmartScreen warning with More info clicked, showing the Run anyway button.

## Installing on Linux

The Linux download can be unpacked anywhere. To run the app from your desktop's app launcher, copy the program, its launcher entry and its icons into your home folder:

```sh
tar -xzf livesplit-asr-bridge-*-x86_64-linux.tar.gz
cd livesplit-asr-bridge
install -Dm755 livesplit-asr-bridge ~/.local/bin/livesplit-asr-bridge
install -Dm644 livesplit-asr-bridge.desktop ~/.local/share/applications/livesplit-asr-bridge.desktop
mkdir -p ~/.local/share/icons
cp -r icons/hicolor ~/.local/share/icons/
```

`~/.local/bin` must be on your `PATH`. Most distributions add it when the folder exists, after you log out and back in. **LiveSplit One ASR Bridge** then appears in the launcher. If the icon doesn't show straight away, log out and back in.

To uninstall, delete the same files:

```sh
rm ~/.local/bin/livesplit-asr-bridge
rm ~/.local/share/applications/livesplit-asr-bridge.desktop
rm ~/.local/share/icons/hicolor/*/apps/livesplit-asr-bridge.png
```

Reading the game's memory can be blocked on Linux. [Platform Notes](Platform-Notes) explains how to allow it.

## Updating

Download the new release and replace the app. Your settings and logs are kept: they live in folders the app doesn't touch. [Logs and Files](Logs-and-Files) lists them.

Next: [Getting Started](Getting-Started).
