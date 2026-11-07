# Running queries and results

## Run
| Command | What it runs |
|---|---|
| **Run**, [[key:run_statement]] | the statement under the cursor; with a selection, the selection |
| **Run all**, [[key:run_script]] | the whole text; with a selection, the selection |
| right-click › **Run selection** | only the selected text |
| **Explain** | the plan of the statement under the cursor: [ER diagram and query plans](er-plans) |
| **Dry run** | the write inside a transaction you then confirm: [Transactions and dry run](transactions) |

If the statement holds placeholders such as `:id`, the parameters bar appears above the results: [Query parameters](parameters).

Before an `UPDATE` or `DELETE` without `WHERE`, a `DROP`, a `TRUNCATE` or an `ALTER`, NiceDB asks for confirmation and tells how many rows would be touched: **Run anyway** goes ahead, **Cancel** backs out. On production connections the rules of [Production safety](production) apply.

After a `CREATE`, `ALTER` or `DROP` the schema reloads on its own.

## Stop a query
Press **Cancel** in the query bar or [[key:cancel]]. The query is cancelled on the server, not just hidden. Rows that already arrived stay in the grid.

With ClickHouse, progress shows while the query runs: rows read, bytes and percentage.

## Result tabs
| Tab | Content |
|---|---|
| **Results** | the grid; with several statements returning rows, **Results 1**, **Results 2**… |
| **Chart** | the rows as a chart: [Value panel and charts](value) |
| **Before and after** | the rows touched by a dry run |
| **Messages** | each statement with its outcome, rows affected and time |
| **Plan** | the plan after **Explain** |

Next to the tabs: row count and time, **Value**, **Copy** and **Export** ([Import and export](import-export)). Right-clicking **Results** offers **Compare with…** ([Tools](tools)) and **Export**.

**Messages** opens on its own when a statement fails or returns no rows. An error shows the server's message, line and column, details and **Hint** when the server gives them; the error spot is also marked in the editor.

## Move and select in the grid
| Key or gesture | Effect |
|---|---|
| arrow keys | next cell |
| [[Ctrl]] + arrow | to the edge of the grid |
| [[PgUp]] / [[PgDn]] | one page |
| [[Home]] / [[End]] | start and end of the row; with [[Ctrl]], first and last cell |
| [[Shift]] + arrows or click | extends the selection |
| [[Ctrl]] + click | adds or removes cells |
| click a row number | selects the row; drag for more rows |
| [[Shift+Space]] / [[Ctrl+Space]] | selects the cell's row / column |
| [[Ctrl+A]] or click the corner | selects everything |
| [[Esc]] | keeps only the active cell |
| right edge of a header | drag to widen the column; double-click to fit the content |
| drag a header | moves the column; a line shows where it will go |
| [[Shift]] + drag a header | selects several columns |

## Find in the grid
With the grid focused, [[Ctrl+F]] opens **Find in results** at the top right. As you type, the cells holding the text (case does not matter) are coloured and the first becomes the active cell; the counter says which one you are on. [[Enter]] and [[Shift+Enter]], or [[F3]] and [[Shift+F3]], go to the next and the previous; [[Esc]] closes and goes back to the grid. It searches only the rows already loaded, up to 100,000 hits, and the value as stored (not the on-screen format of dates and numbers).

## Hidden personal data
With **Settings › Results › Personal data › Hidden** (off by default), columns that look like tax codes, IBANs, emails or phone numbers show `••••••`. They are recognised as in the Security lab: by the column name, confirmed by at least one of the first values. The eye in the header, or right-click › **Show values**, reveals them for that result; **Hide values** covers them again. It is meant for a shared screen: copy, export, the value panel and editing use the real value, and Ctrl+F does not search hidden columns.

## Columns
Right-click a header:
- **Freeze on the left** keeps the column in place while you scroll sideways, for example the primary key; more than one can be frozen. **Unfreeze column** puts it back after the frozen ones.
- **Fit width** and **Fit all columns** measure the loaded values.
- **Reset order and widths** goes back to the query's order.

Order, frozen columns and widths set by hand are remembered per table, on this computer: they apply in the **Data** view and in query results on that table, by column name.

[[Shift+Enter]] opens the value panel for the cell: [Value panel and charts](value). Right-clicking a cell offers copying, sorting, selection and **Compare with…**.

## Selection totals
With at least two cells selected, the status bar shows **Count**, **Sum**, **Average**, **Min**, **Max** and **Distinct**. Sums are exact even with many decimals. Clicking a value copies it.

## Copy
- [[Ctrl+C]] copies the selection as tab-separated text.
- **Copy** above the grid: **As text (TSV)**, **As CSV**, **As JSON**, **As INSERT**.
- Right-click › **Copy as**: **Text with headers**, CSV, JSON, INSERT; then **Copy column** and, on the header, **Copy name**.

## Sort
Clicking a header sorts the column: ascending, descending, then unsorted. [[Shift]] + click adds the column to the sort. The same items are in the right-click menu: **Sort ascending**, **Sort descending**, **Remove sort**.

Sorting happens on the server: NiceDB runs the statement again with an `ORDER BY` and swaps the rows in. [[Ctrl+Z]] goes back to the previous order. A result that is not from a `SELECT`, that writes, or that comes from several statements at once cannot be sorted: the reason shows above the grid.

## Long results
- **Settings › Results › Row cap per query** stops reading past a number of rows: "Showing the first N rows: narrow the query to see them all."
- The connection's **Row limit** cuts `SELECT`s without `LIMIT`: "Connection limit: showing the first N rows" appears with **Load all**, which reads the statement again without that limit and keeps the sort. See [Connections and profiles](connections).

## Dropped connection
If the connection drops during a query, a notice appears above the results. The query is never run again on its own:
- for a read, **Run again from here** runs it again safely;
- for a write the notice says it may have run, and the button is **Run again anyway**;
- if the connection does not come back, **Reconnect** tries again.

## History
**History** in the query bar, or [[key:panel_history]], opens the list of queries run on this connection, with date, time and rows. Failed queries are marked.
- **Search history** filters by text.
- **Connections**: **This one** shows only the tab's connection, **All** shows every connection, with its name before the date.
- **Outcome**: **All**, **Succeeded** or **Failed**.
- With the panel open, each run that ends shows up by itself, within the search and filters.
- A click inserts the query at the end of the editor.
- Right-click: **Insert into editor**, **Open in a new tab** (on the connection where it ran), **Copy**.

## Limitations
- The results grid is read-only: to change rows use the **Data** view ([Data view and editing](data)).
- Sorting runs the query again: on a slow query the new order takes just as long.
- MySQL cannot sort a result with two columns of the same name.
- The connection limit stops reading by cancelling the query: the server may have computed more rows already.
