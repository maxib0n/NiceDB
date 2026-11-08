# Changelog

## 0.3.2 (2026-11-08)

- Locked production: the grid menu shows write items greyed out, with the reason.
- A failed Apply names the constraint, table and columns (and the missing side of a foreign key).
- Leaving a table with pending edits asks to apply, discard or stay.
- No more slowdown when selecting text in long scripts.
- Full version in the status bar; the foreign key arrow stays inside its cell; aligned Links icons with clearer tooltips.
- No pointer prompt after updating: choose pointers in Settings › Appearance.

## 0.3.1 (2026-11-07)

- The first start follows the system: interface language (Italian, Chinese, otherwise English) and light or dark theme.
- The window fits smaller screens: the minimum size is now 960×600.
- A bug report template asks for the engine and version, steps and expected result.

## 0.3.0 (2026-11-07)

- The ER diagram legend is available in every style.
- Running something while a dry run still waits for Commit or Rollback rolls the dry run back first, instead of refusing; an open-transaction error offers Commit and Rollback in place.
- The history panel shows new runs while it is open.
- `nicedb run --conn <connection> file.sql --csv`: runs SQL from the command line on saved connections, with passwords from the keychain and the production gate.
- Presentation mode (Ctrl+Alt+P): larger type, personal data and connection addresses hidden, for videos and demos.
- Dry runs and write confirmations say what a DELETE, TRUNCATE or DROP TABLE would take away, and the rows of other tables pointing at it (cascade, set null or blocking).
- The write log can put back the old values of changes applied from the Data view, after showing the SQL and through the production gate; it says which writes cannot be reverted.
- Import a JSON, Extended JSON or NDJSON file into a MongoDB collection, an Elasticsearch index or a DynamoDB table, with a preview, a count and the lines left out.
- Plan history: every Explain is kept per query (values ignored), with a warning when the shape changes or it gets more than 50% slower, and any saved plan can be compared with the current one.
- Grid: drag a header to move the column, freeze columns on the left, fit all columns at once; order, frozen columns and widths are remembered per table.
- Optional masking of personal data in results (tax codes, IBANs, emails, phone numbers), off by default; the header eye reveals a column.
- Ctrl+F in the results grid and the Data view highlights matching cells and steps through them.
- Ctrl+Shift+T reopens the last closed tab, and the ones before it, with their text.
- Connections can have a colour and a mark, shown in the tree and on their tabs; eight new marks for folders and connections.
- Open and save `.sql` files: a File button in the editor bar, Ctrl+O, Ctrl+S and Ctrl+Alt+S (save as), starting from the last folder used; drop `.sql` files on the window to open them.
- Saved scripts and bookmarked queries can be inserted into the editor from Ctrl+K.
- History can show every connection and only successful or failed runs.
- Tabs are saved when the window closes too, so text typed just before quitting comes back at the next start.
- The connection tree indents each level clearly under its connection.
- Thinner cell and crosshair pointers.

## 0.2.2 (2026-11-06)

- Simplified Chinese interface, contributed by @YNM9000.
- Pointers in the style: arrow, text cursor and handles drawn in each style's shape and colours, with a size setting. Existing installs are asked once; Settings › Appearance turns them off.
- Switching an open connection to production locks it and shows the unlock control at once, without reopening.
- Update notes in English for languages other than Italian.

## 0.2.1 (2026-11-05)

- The status bar shows the right version.

## 0.2.0 (2026-11-05)

- Production is harder to miss: remote hosts suggest the production flag, production tabs and the title bar are marked, and an unlocked session shows its countdown on every tab that shares it.
- Workspace exports carry only database passwords. SSH, proxy and certificate secrets stay with you.
- Linux: works with a keyring that has no default collection, and says how to create one when there is none.
- Manual: the write log is kept by NiceDB on your computer, not a server audit.

## 0.1.0 (2026-11-01)

- First public release.
