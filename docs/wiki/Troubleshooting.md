# Troubleshooting

## LiveSplit One won't connect

**What you see:** LiveSplit One can't connect, and the Timer card says **NOT CONNECTED**.

**Why:** The address is wrong or isn't reachable from the timer PC, a firewall blocks the port, Chrome blocked local network access, or the browser isn't Chrome-based.

**What to do:**

1. Use the address labelled **LAN** or **VPN** that matches how the timer PC reaches the game PC. See [Connecting LiveSplit One](Connecting-LiveSplit-One).
2. Allow the port, `16834` by default, in the game PC's firewall.
3. Allow local network access for LiveSplit One in Chrome's site settings. The steps are on [Connecting LiveSplit One](Connecting-LiveSplit-One).
4. Use Chrome, Edge, Brave or another Chrome-based browser.
5. Click **Restart server** in the **Connection** tab, then connect again.

## The Game card stays on WAITING FOR GAME…

**What you see:** The game is running, but the Game card says **WAITING FOR GAME…**.

**Why:** The game isn't the one the auto splitter is for, or the app isn't allowed to read its memory.

**What to do:**

- Check the loaded file is the auto splitter for your game and version.
- On Linux, check `ptrace_scope`. See [Platform Notes](Platform-Notes).
- On macOS, reading a game needs extra permissions. See [Platform Notes](Platform-Notes).

## Last action says "Not sent: no timer connected"

**What you see:** Actions show under **Last action** with the note "Not sent: no timer connected".

**Why:** The auto splitter works. No LiveSplit One is connected, so the commands were dropped.

**What to do:** Connect LiveSplit One. See [Connecting LiveSplit One](Connecting-LiveSplit-One).

<!-- screenshot: S9 not-sent.png -->
The Last action card showing SPLIT with the amber note "Not sent: no timer connected".

## "Port 16834 is already in use"

**What you see:** The Error card says "Port 16834 is already in use. Choose another port in Connection and restart the server." The Timer card says **SERVER STOPPED**.

**Why:** Another program, or a second copy of this app, is using the port.

**What to do:** Click **Open Connection**, type another number in **Port**, and click **Restart server**. Then use the new address in LiveSplit One.

<!-- screenshot: S8 port-in-use.png -->
The Error card with the port-in-use message and the Open Connection and Dismiss buttons, above a Timer card showing SERVER STOPPED.

## "The auto splitter stopped because of an error."

**What you see:** The Error card says "The auto splitter stopped because of an error. Press Reload to start it again." The Game card says **STOPPED**.

**Why:** The auto splitter crashed.

**What to do:** Click **Show in log** to read what it printed before it stopped. Click **Reload** to start it again. If it keeps happening, report it to the auto splitter's author with the log lines.

## "Couldn't load … not a valid WebAssembly module"

**What you see:** The Error card says "Couldn't load foo.wasm: not a valid WebAssembly module." If an auto splitter was running, it keeps running.

**Why:** The file isn't a `.wasm` auto splitter. It may be an older `.asl` script, or a download that broke.

**What to do:** Get a `.wasm` auto splitter for the Auto Splitting Runtime, and download it again if needed. See [Auto Splitters and Settings](Auto-Splitters-and-Settings). **Show in log** has more detail.

## The timer doesn't split, but actions show

**What you see:** **Last action** updates, and LiveSplit One doesn't react.

**Why:** LiveSplit One has no run in progress, or rejected the command.

**What to do:** Check LiveSplit One has a run loaded. Tick **Connection** in the **Log** tab to see which commands it rejected. See [Logs and Files](Logs-and-Files).

## The OS won't open the app

**What you see:** macOS can't verify the developer, or Windows says "Windows protected your PC".

**Why:** Releases aren't code-signed yet.

**What to do:** See [Installing](Installing).

## Reporting a bug

1. Open **Preferences** and note the version under **About**.
2. In the **Log** tab, tick every category and reproduce the problem.
3. Find the day's log file. [Logs and Files](Logs-and-Files) says where.
4. Open an issue on [the project's GitHub page](https://github.com/alexcosta97/livesplit-asr-bridge/issues/new/choose) with the Bug report form. Attach the log file and say what you expected.
