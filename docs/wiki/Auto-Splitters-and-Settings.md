## Getting an auto splitter

LiveSplit One ASR Bridge runs `.wasm` auto splitters, the kind built for LiveSplit's Auto Splitting Runtime. It can't run the older script-based `.asl` files.

Get the `.wasm` file from the auto splitter's author, or from LiveSplit's list of auto splitters. The app doesn't download them yet.

## Opening and reloading

On the Auto splitter card, click **Open…** and pick the `.wasm` file. The card then shows the file name and the game it's for. The app remembers the file and loads it again the next time it starts.

Click **Reload** to load the same file again. Use it after the file was rebuilt, or when the auto splitter stopped because of an error. **Reload** is disabled until a file is loaded.

## Which game it's for

The first time you open a file, the app asks "Which game is this auto splitter for?". It fills in a name from the file name. Change it, or pick a game under **Games already set up**, then click **Use this game**. **Cancel** doesn't load the file.

<!-- screenshot: S3 game-dialog.png -->
The dialog "Which game is this auto splitter for?" with a name field, the list of games already set up, and the Use this game and Cancel buttons.

The app saves your settings for each game, not for each file. It asks once per file and remembers the answer. To change it later, click **Change** on the Auto splitter card. That opens the same dialog, titled "Change game".

## Changing settings

The **Splitter settings** tab shows the settings the auto splitter publishes: headings, checkboxes, dropdowns, text fields and file pickers. Hover the info marker next to a setting to read its description.

<!-- screenshot: S4 splitter-settings.png -->
The Splitter settings tab with a heading, checkboxes, a dropdown and a text field, "● Unsaved changes" in the toolbar and a dot on the tab label.

Your edits are drafts. The running auto splitter keeps its current values until you click **Save**, so a stray click mid-run changes nothing. While there are drafts:

- The toolbar shows "● Unsaved changes".
- The tab label shows a `•`.
- **Save** is enabled.

After you save, the toolbar shows "✓ Saved" for a moment.

**Revert to defaults** puts the auto splitter's default values back as drafts. It's a draft too: click **Save** to use them.

A text field shows "Default: …" and **Use default** under it when its text differs from the default. **Use default** puts the default text back as a draft. An empty field is a value of its own, not the default.

A file setting shows the picked path, a ✕ to clear it, and **Browse…**.

### Unsaved changes

If you close the app, open another file, click **Reload** or click **Change** while there are unsaved changes, the app asks "Save your settings changes?". It says how many settings changed and what discards them, for example "You changed 3 settings for GTA San Andreas. Reloading the auto splitter without saving discards them."

- **Cancel**: go back, change nothing.
- **Discard**: drop the drafts and carry on.
- **Save and reload**, or **Save** for the other cases: save, then carry on.

When you open another file, this comes before the file picker.

## Settings shared by a game

All auto splitters for the same game share one set of saved values. If two of them have a setting with the same name, they share its value. Settings with other names are left alone, and saving never removes another auto splitter's values.

Auto splitters can also store values themselves. The app saves those too, so nothing is lost when you close it.

## Developer mode

If you write or debug auto splitters, turn on **Developer mode** in the **Preferences** tab. The tab says:

> Shows the auto splitter's settings map in Splitter settings, and its messages in Log. For writing or debugging auto splitters.

In **Splitter settings**, a "Settings map" section appears under the settings. It lists every key the running auto splitter holds, with its type and value, read only. A value the auto splitter just changed is tinted for a few seconds and marked "changed". **Hide** and **Show** collapse the section.

In **Log**, the **Auto splitter** filter is ticked. See [Logs and Files](Logs-and-Files).

Turning Developer mode off puts the **Auto splitter** filter back as it was.

Developer mode doesn't change how the auto splitter runs.

## Next

- [The Main Window](The-Main-Window): the Auto splitter card and the other cards.
- [Troubleshooting](Troubleshooting): if a file won't load.
