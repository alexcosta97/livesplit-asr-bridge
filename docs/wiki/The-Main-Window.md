# The Main Window

## Layout

The window has two parts. The status column on the left is always visible. It shows the app name and version, then cards for the Auto splitter, the Game, the Timer and the Last action. On the right are four tabs: **Splitter settings**, **Connection**, **Log** and **Preferences**.

<!-- screenshot: S1 main-window.png -->
The main window with an auto splitter loaded, the Game card showing ATTACHED, the Timer card showing CONNECTED and a split in Last action.

Status words are colour-coded: green for working, grey for waiting, red for errors. They're drawn in capitals.

The smallest supported window is 960 px wide. Below about 640 px, the window switches to the [compact window](#compact-window).

## Error card

The Error card shows only while there's an error, at the top of the column. It shows the most recent error until you click **Dismiss**.

| You see | What it means | What to do |
|---|---|---|
| "The auto splitter stopped because of an error. Press Reload to start it again." | The auto splitter crashed. | **Show in log** for the details, then **Reload**. |
| "Couldn't load foo.wasm: …" | The file didn't load. If one was running, the card adds "The previous auto splitter is still running." | **Show in log**. See [Troubleshooting](Troubleshooting). |
| "Port 16834 is already in use. Choose another port in Connection and restart the server." | Another program is using the port. | **Open Connection**, pick another port, **Restart server**. |
| "Couldn't start the server on port …" | The server couldn't listen for another reason. | **Open Connection**, try another port. |

**Show in log** opens the **Log** tab with the right category ticked, at the line that explains the error.

## Auto splitter card

Shows the file name and its game, with **Change**, **Open…** and **Reload**. With nothing loaded it says "NO AUTO SPLITTER" and "Open a .wasm auto splitter to start.". [Auto Splitters and Settings](Auto-Splitters-and-Settings) covers all of these.

## Game card

| Status | What it means | What to do |
|---|---|---|
| **ATTACHED** | The auto splitter found the game. It shows the process name and how often the auto splitter runs, for example "SanAndreas.exe · 20 Hz". | Nothing. |
| **WAITING FOR GAME…** | The auto splitter runs, but the game isn't running. This is normal before a run. | Start the game. If it stays, see [Troubleshooting](Troubleshooting). |
| **STOPPED** | The auto splitter crashed and isn't running. | Click **Reload** in the Error card or on the Auto splitter card. |
| **—** | No auto splitter is loaded. | Click **Open…**. |

## Timer card

| Status | What it means | What to do |
|---|---|---|
| **NOT CONNECTED** | No LiveSplit One is connected. The card lists the addresses to connect to, each with **Copy**. | Click **How do I connect?**, or see [Connecting LiveSplit One](Connecting-LiveSplit-One). |
| **CONNECTED** | At least one LiveSplit One is connected. The card shows how many, for example "2 timers · LiveSplit One". | Nothing. |
| **SERVER STOPPED** | The app can't listen for connections, so no timer can connect. The Error card says why. | Pick another port and click **Restart server** in the **Connection** tab. |

Where the addresses go while a timer is connected, and how to use them, is on [Connecting LiveSplit One](Connecting-LiveSplit-One).

## Last action card

Shows the latest command the auto splitter sent, in large type, with its time: START, SPLIT, SKIP SPLIT, UNDO SPLIT, RESET, GAME TIME, PAUSE GAME TIME or RESUME GAME TIME. A split shows the segment's name when LiveSplit One provides it. Two earlier actions show below it. These are kept for this session only.

The game time is often set on every tick, so a GAME TIME right after another GAME TIME replaces it rather than adding a line.

Before anything happens, the card says "No actions yet".

If an action happens while no timer is connected, the card shows it with the note "Not sent: no timer connected". The command was dropped. The auto splitter works. The problem is the connection.

<!-- screenshot: S9 not-sent.png -->
The Last action card showing SPLIT with the amber note "Not sent: no timer connected".

To fix it, see [Connecting LiveSplit One](Connecting-LiveSplit-One).

## Compact window

Below about 640 px wide, the tabs are hidden and the status column fills the window. The status words get bigger, so you can check the run at a glance. A thin strip replaces the Auto splitter card, with the file name and **Reload**, or "No auto splitter loaded" and **Open…**. The Timer card shows one address while no timer is connected.

<!-- screenshot: S6 compact-window.png -->
The compact window: the app name, the splitter strip, and the Game, Timer and Last action cards.

Click **Show details** to see the tabs. The window gets wider if your window manager allows it. If not, for example in a tiling window manager, the tabs replace the status column in the same space, and **← Status** takes you back. **Show in log** and **Open Connection** show the tabs the same way.

## Preferences

The **Preferences** tab has:

- **Remember window size and position**, on by default. The app saves the window's size and position and restores them. The tab suggests turning it off with a tiling window manager, so the window manager decides the size.
- **Developer mode**. See [Auto Splitters and Settings](Auto-Splitters-and-Settings).
- **About**: the app version, and the **Config folder** and **Log folder**, each with an **Open** button. [Logs and Files](Logs-and-Files) says what's in them.
