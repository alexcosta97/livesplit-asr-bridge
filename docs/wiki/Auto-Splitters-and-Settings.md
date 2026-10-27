## Getting an auto splitter

LiveSplit One ASR Bridge runs `.wasm` auto splitters, the kind built for LiveSplit's Auto Splitting Runtime. It can't run the older script-based `.asl` files.

Get the `.wasm` file from the auto splitter's author, or from LiveSplit's list of auto splitters. The app doesn't download them yet.

## Opening and reloading

On the Auto splitter card, click **Open…** and pick the `.wasm` file. The card then shows the file name, the game it's for and the category whose settings are in use. The app remembers the file and loads it again the next time it starts.

Click **Reload** to load the same file again. Use it after the file was rebuilt, or when the auto splitter stopped because of an error. **Reload** is disabled until a file is loaded.

## Which game it's for

The first time you open a file, the app asks "Which game is this auto splitter for?". It fills in a name from the file name. Change it, or pick a game under **Games already set up**, then click **Use this game**. **Cancel** doesn't load the file.

<!-- screenshot: S3 game-dialog.png -->
The dialog "Which game is this auto splitter for?" with a name field, the list of games already set up, and the Use this game and Cancel buttons.

The app saves your settings for each game and category, not for each file. It asks once per file and remembers the answer. To change it later, click **Change** on the Auto splitter card. That opens the same dialog, titled "Change game".

### Linking a game to speedrun.com

The name field also searches speedrun.com, so you can link the game to its speedrun.com page instead of retyping its name.

- **When the dialog opens**, it looks up the name it filled in from the file. In "Change game" it looks up the current game, if that game isn't linked yet. If exactly one result clearly matches, the app links it on its own. Otherwise the list of suggestions opens for you to choose from. Picking a game under **Games already set up** that isn't linked yet does the same lookup.
- **While you type**, suggestions appear after a short pause. A name you type is never linked on its own; you pick the row. The list shows up to five rows and scrolls for more. Each row shows the year the game came out, and **SET UP** marks a speedrun.com game already linked to a game you set up. Use the arrow keys to move through the list, **Enter** to pick a row and **Escape** to close it.
- **The last row**, "Use "&lt;name&gt;" (not on speedrun.com)", keeps the name you typed with no link.

A linked game shows "Linked to speedrun.com" with a **Change** button. **Change** removes the link and reopens the list. Using a game without its link unlinks it, and it keeps its name.

Linking renames the game to speedrun.com's name. Its settings and categories aren't affected. If you pick a speedrun.com game that another game is already linked to, the app uses that game, so you don't end up with two. The same goes for a game you set up under speedrun.com's name before: the app uses and links it.

**What is sent to speedrun.com.** Only while the Game dialog is open, the names in its field are sent to speedrun.com over HTTPS. While the Category dialog is open for a linked game, the game's speedrun.com id is sent to ask for its categories ([below](#picking-a-category-from-speedruncom)). No account is needed, and nothing else is sent. If you're offline or speedrun.com doesn't answer, the dialog works as before without suggestions, and the **App & runtime** log notes it once. See [Troubleshooting](Troubleshooting).

## Categories

Each game has one or more categories, such as Any% and 100%, and each category keeps its own settings. That way the same auto splitter can split on different things for different runs. A new game starts with one category, "Default". If you used the app before categories existed, the app asks "Name this category" when it loads a game with older settings. That happens when you load an auto splitter, when the app starts and when you change its game. The dialog asks which category those settings are for, such as Any%. Click **Save** to name the category after your answer and save it. Choose **Not now** to keep the name "Default"; it asks again next time, until the game's file is next written (for example when you save settings), and you can rename the category later.

The Auto splitter card shows the category in use under the game. To change it, click the **Change** next to the category. The Category dialog lists the game's categories, and the one in use is marked IN USE.

- **Use another category:** select it and click **Use this category**. The auto splitter keeps running with that category's settings. If you have unsaved changes, the app asks about them first.
- **Add one:** click **+ New category**, type a name and click **Add**. It starts as a copy of the settings in use now, without unsaved changes. The new category is selected, but the app doesn't switch to it. Click **Use this category** to do that.
- **Rename one:** click **Rename** on its row, type the new name, then press Enter or click **Done**. Escape keeps the old name.
- **Delete one:** select it, click **Delete**, then **Delete** again. The dialog asks first; click **Keep** to back out. Deleting can't be undone. You can't delete the category in use or a game's last category.

A category name can't be empty, and can't match another category of the same game, ignoring upper and lower case. **Add** and **Done** stay disabled until the name is valid.

Adding, renaming and deleting are saved straight away. **Cancel** only closes the dialog. The app remembers the last category you used for each game and starts with it next time.

<!-- screenshot: S10 category-dialog.png -->
The Category dialog with three categories, Any% marked IN USE, and the + New category, Delete, Cancel and Use this category buttons.

LiveSplit One doesn't tell the app which category your splits are for, so choose it here.

### Picking a category from speedrun.com

When the game is linked to speedrun.com (see [above](#linking-a-game-to-speedruncom)), the category name fields offer the game's categories from speedrun.com, so you can link a category to its page instead of retyping its name. The fields are the ones for **+ New category**, **Rename** and "Name this category". A game that isn't linked has no list and works as before.

- **The list.** A list drops down under the field while it has focus. It shows up to five rows and scrolls for more. Each row is a category as speedrun.com has it, with its subcategories in parentheses, such as "Any% (Unrestricted)". **SET UP** marks one a category of this game is already linked to. Typing narrows the list to the rows whose name contains what you typed, ignoring case and anything that isn't a letter or digit. Use the arrow keys to move through the list, **Enter** to pick a row and **Escape** to close it.
- **A few categories or many.** With 20 or fewer, the empty field lists them all, so you can pick without typing. With more, the list has one muted row, "Type to search &lt;count&gt; categories", and the categories appear once you type.
- **Level categories.** If the game has categories for single levels, a checkbox **Include level categories** appears under the field. It's unticked unless the category you're renaming is linked to a level. Tick it to add them to the list, with the level's name first, such as "Bob-omb Battlefield: Stage RTA (N64)".
- **The last row.** Once you've typed something, the last row, "Use "&lt;name&gt;" (not on speedrun.com)", keeps the name you typed with no link. While the categories load, and when you're offline, it's the only row.

Picking a category fills the field with its name and links it. The field shows "Linked to speedrun.com" with a **Change** button. Typing in the field removes the link until you pick a category again. **Change** removes the link and opens the list.

- **Adding.** If you pick a category that another category of the game is already linked to, or one that has its name, the app uses that category instead of adding another. It links it, takes the name from speedrun.com, saves it and selects it.
- **Renaming and naming.** Those categories can't be picked, and their rows are dimmed, so two categories never share a link or a name. When you add, a row is dimmed if either category would give two the same name or link.
- **Rename** on a category that isn't linked looks at its name and links it on its own if one category clearly matches. Otherwise the list opens. Names you type, and the empty fields of **+ New category** and "Name this category", are never linked on their own. **Done** keeps the name and the link. If you type a different name instead, the category is unlinked and keeps what you typed.

Linking renames the category to speedrun.com's name. Its settings aren't affected. If you unlink the game, or change it to a different speedrun.com game in the Game dialog, its categories' links are removed, since they pointed to the old game. They keep their names and settings.

Offline, or if speedrun.com doesn't answer, the dialog works as before without the list, and the **App & runtime** log notes it once for that dialog. See [Troubleshooting](Troubleshooting).

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

If you close the app, open another file, click **Reload**, click **Change** for the game, or use another category while there are unsaved changes, the app asks "Save your settings changes?". It says how many settings changed and what discards them, for example "You changed 3 settings for GTA San Andreas (Any%). Reloading the auto splitter without saving discards them."

- **Cancel**: go back, change nothing.
- **Discard**: drop the drafts and carry on.
- **Save and reload**, or **Save** for the other cases: save, then carry on.

When you open another file, this comes before the file picker.

## Settings shared by a game and category

All auto splitters for the same game share one set of saved values for each category. If two of them have a setting with the same name, they share its value. Settings with other names are left alone, and saving never removes another auto splitter's values.

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
