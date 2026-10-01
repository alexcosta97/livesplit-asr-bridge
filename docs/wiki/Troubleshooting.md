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

**What you see:** The Game card says **WAITING FOR GAME…**.

**Why:** The game isn't running, isn't the one the auto splitter is for, or the app isn't allowed to read its memory.

**What to do:**

- Start the game, and check it's really running.
- Check the loaded file is the auto splitter for your game and version.
- On Linux, check `ptrace_scope`. See [Platform Notes](Platform-Notes).
- On macOS, reading a game needs extra permissions. See [Platform Notes](Platform-Notes).

## Last action says "Not sent: no timer connected"

**What you see:** Actions show under **Last action** with the note "Not sent: no timer connected".

**Why:** No LiveSplit One is connected, so the commands were dropped.

**What to do:** Connect LiveSplit One. [The Main Window](The-Main-Window) has the detail, and [Connecting LiveSplit One](Connecting-LiveSplit-One) has the steps.

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

## "Couldn't load …"

**What you see:** The Error card says "Couldn't load foo.wasm: failed loading the WebAssembly module." or "Couldn't load foo.wasm: couldn't read the file." If an auto splitter was running, the card adds "The previous auto splitter is still running."

**Why:** "Failed loading the WebAssembly module" means the file isn't a valid `.wasm` auto splitter. It may be an older `.asl` script, or a download that broke. "Couldn't read the file" means the file is missing or the app can't read it.

**What to do:** For the first message, get a `.wasm` auto splitter for the Auto Splitting Runtime, and download it again if needed. For the second, check that the file is still there and that you can open it. See [Auto Splitters and Settings](Auto-Splitters-and-Settings). **Show in log** has more detail.

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
2. Reproduce the problem. Every category is written to the log file whatever the **Log** tab filters show, so you don't need to tick anything for the report.
3. Find the day's log file. [Logs and Files](Logs-and-Files) says where.
4. Open an issue on [the project's GitHub page](https://github.com/alexcosta97/livesplit-asr-bridge/issues/new/choose) with the Bug report form. Attach the log file and say what you expected.
