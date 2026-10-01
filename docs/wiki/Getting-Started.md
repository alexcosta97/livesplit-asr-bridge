# Getting Started

## What you need

- The game PC: the PC your game runs on, with LiveSplit One ASR Bridge [installed](Installing).
- The timer PC: a PC with a Chrome-based browser (Chrome, Edge, Brave and the like) to run LiveSplit One.
- A `.wasm` auto splitter for your game.
- Both PCs on the same network, or on the same VPN.

## From first launch to your first split

1. **Open the app.** The first time, it shows the **Connection** tab with the setup steps.

   <!-- screenshot: S2 first-launch.png -->
   The window on first launch, on the Connection tab with the How to connect steps visible and No timers connected yet above them.

2. **Click Open…** and pick the `.wasm` auto splitter.

3. **Name the game.** The app asks "Which game is this auto splitter for?" and fills in a name from the file name. Change it if it's wrong, or pick a game from the list, then click **Use this game**. The app remembers the game's settings.

   <!-- screenshot: S3 game-dialog.png -->
   The dialog "Which game is this auto splitter for?" with a name field, the list of games already set up, and the Use this game and Cancel buttons.

4. **Connect LiveSplit One.** On the Timer card, or in the **Connection** tab, click **Copy** next to an address. In LiveSplit One, open Connect to Server and paste it. When Chrome asks, allow local network access. [Connecting LiveSplit One](Connecting-LiveSplit-One) has the details.

   <!-- screenshot: T1 lso-connect-to-server.png -->
   LiveSplit One's Connect to Server dialog with the address pasted in.

5. **Start the game.** The Game card shows **ATTACHED** and the Timer card shows **CONNECTED**. When the auto splitter starts, splits or resets, the action appears under **Last action**, and LiveSplit One follows.

   <!-- screenshot: S1 main-window.png -->
   The main window with the Game card showing ATTACHED, the Timer card showing CONNECTED and a split under Last action.

The Game card says **WAITING FOR GAME…** until the game runs. That's normal.

## Next

- [Connecting LiveSplit One](Connecting-LiveSplit-One): several addresses, several timers, the port.
- [Auto Splitters and Settings](Auto-Splitters-and-Settings): auto splitter settings and saving them.
- [The Main Window](The-Main-Window): what every card means.
- [Troubleshooting](Troubleshooting): if something doesn't work.
