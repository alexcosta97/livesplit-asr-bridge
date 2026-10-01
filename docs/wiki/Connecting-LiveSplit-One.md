# Connecting LiveSplit One

## Which address to use

The app lists one address for each network address of the game PC, each labelled **LAN** or **VPN**, for example `ws://192.168.1.20:16834`. Use the one the timer PC can reach: a LAN address when both PCs are on the same network, a VPN address when they're on the same VPN. The port, `16834` by default, is part of the address.

Addresses show on the Timer card and in the **Connection** tab, each with **Copy**. They're hidden on the Timer card while a timer is connected, and stay in the **Connection** tab.

## Connecting

LiveSplit One must run in a Chrome-based browser (Chrome, Edge, Brave and the like). Other browsers can't connect yet.

The **Connection** tab shows the steps under **How to connect**:

1. Copy an address.
2. In LiveSplit One, open **Settings → Connect to Server** and paste it.
3. When Chrome asks, allow local network access.

<!-- screenshot: T1 lso-connect-to-server.png -->
LiveSplit One's **Connect to Server** dialog with the address pasted in.

The first time, Chrome asks whether LiveSplit One may access devices on your local network. Click **Allow**. Without it, the connection can't be made.

<!-- screenshot: T2 chrome-local-network.png -->
Chrome's prompt asking to allow local network access for the LiveSplit One site, with the Allow button.

If you blocked it by mistake, Chrome remembers. Open the site's settings from the lock icon in the address bar and set Local network access to Allow.

<!-- screenshot: T2b chrome-site-settings.png -->
Chrome's site settings for the LiveSplit One site, with Local network access set to Allow.

Once a timer is connected, the Timer card shows **CONNECTED** and the steps collapse. Click **Show setup steps** to see them again, and **Hide setup steps** to collapse them.

## Several timers

Every connected timer gets every command, for example two browser tabs. The **Connected timers** list shows each one's address and its state, like "Running · split 12".

The **PRIMARY** tag marks the timer the app follows to know the timer's state, for auto splitters that ask about it. It's the first one that connected. When it disconnects, the next one that's been connected longest takes over.

<!-- screenshot: S5 connection-tab.png -->
The Connection tab with two timers listed under Connected timers, one tagged PRIMARY, and the Server section with the Port field.

## Changing the port

In the **Connection** tab, type a new number in **Port** (1 to 65535). The app shows "Restart the server to apply". Click **Restart server**. That closes every connection and listens again on the new port, and the addresses change to match. Then connect LiveSplit One again.

**Restart server** is always available. It's also a good first thing to try when LiveSplit One won't connect. If the port is in use by another program, the app says so: choose another port and restart the server. [Troubleshooting](Troubleshooting) covers more.

## Who can connect

The app listens on every network interface, so another machine can reach it. Anyone on the same network, or the same VPN, who has the address can connect and receive the timer commands. The commands hold nothing sensitive, and a connected timer can only change what the app thinks the timer's state is, not anything on your PC.
