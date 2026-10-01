# Platform Notes

## Linux

The app reads the game's memory. Linux can block that with a security setting called `ptrace_scope`. Check it:

```sh
cat /proc/sys/kernel/yama/ptrace_scope
```

- `0`: nothing to do.
- `1` or higher: the app may not be able to attach to the game. The Game card stays on **WAITING FOR GAME…** while the game runs. Use one of the options below.

Games running under Proton or Wine are read like any other program. The same fix applies.

### Option A: allow the app

This lets only this app read other programs' memory. It's the safer option. Run it once, with the real path of the program. If you installed it as [Installing](Installing) describes, that's in your home folder:

```sh
sudo setcap cap_sys_ptrace=eip ~/.local/bin/livesplit-asr-bridge
```

If you run the app from the unpacked download instead, give that path. Repeat the command after each update, because a new file loses the permission.

### Option B: relax it for every program

This lasts until you reboot. It weakens a security protection for every program on the PC:

```sh
sudo sysctl kernel.yama.ptrace_scope=0
```

To keep it after a reboot, write it to a file and reload:

```sh
echo 'kernel.yama.ptrace_scope = 0' | sudo tee /etc/sysctl.d/99-ptrace.conf
sudo sysctl --system
```

To undo it, delete that file and reboot.

## macOS

Reading a game's memory needs extra permissions on macOS, and may need the app to run with elevated rights. macOS is supported, but it's the least tested platform for the game PC. If the Game card stays on **WAITING FOR GAME…**, this is the first thing to suspect.

The first launch needs a warning dismissed. See [Installing](Installing).

## Windows

No setup is needed for reading the game. The first launch shows a SmartScreen warning. See [Installing](Installing).

## Browser

LiveSplit One must run in a Chrome-based browser (Chrome, Edge, Brave and the like). Chrome asks for local network access the first time. Allow it. [Connecting LiveSplit One](Connecting-LiveSplit-One) has the steps.
