# Import and export

## Export a table
1. Right-click the table in the explorer › **Export data…**. Alternatively: **Tools › Import and export**, pick the table and press **Export…**; or the palette command “Export *table* to…”, which asks for the format straight away.
2. Choose the **Format**:

   | Format | Good for |
   |---|---|
   | CSV | commas, for any program |
   | TSV | tabs, for pasting |
   | JSON | numbers and JSON keep their type |
   | XLSX | one sheet, numbers as numbers |
   | Parquet | columnar and compressed |

3. For CSV, TSV and XLSX keep or clear **First row with column names**.
4. **Export…** and choose where to save the file.

- The file is written while the table is read: there is no row limit in memory.
- The bar shows progress; **Cancel** stops everything and deletes the partial file.
- At the end: rows written, file name, size and time.
- An Excel sheet holds at most 1,048,576 rows: beyond that, use CSV or Parquet.

## Export a query result
1. Run the query.
2. **Export** above the results, or right-click the results tab › **Export**.
3. Choose format and file as for a table.

The query runs again on a separate connection and is written as it arrives, so the file holds every row, not just those in the grid. Only a `SELECT` result can be exported.

For a few rows the grid is enough: right-click › **Copy as** › **Text with headers**, **As CSV**, **As JSON** or **As INSERT**. See [Running queries and results](results).

## Import a file into a table
1. Open the import:
   - right-click a table › **Import data…**, to import into that table;
   - **Tools › Import and export › Import a file…**;
   - the palette command “Import data into *table*…”.
2. **Choose a file…**: CSV and TSV (delimiter detected, semicolons too), JSON (an array of objects or one object per line), XLSX, Parquet. The file is never loaded whole into memory.
3. Check the preview. If needed, change **First row has names** and, for an Excel file, the **Sheet**.
4. Choose the **Target**:
   - **Existing table**: in **Column mapping** each table column takes a file column, matched by name where possible; “— leave empty” skips it.
   - **New table**: type the **Table name**; for each file column choose whether to import it, its **Column name** and the proposed **Type**.
5. Keep or clear **Empty cells as NULL**.
6. Choose what to do with **Rows with errors**:
   - **Stop at the first error**: one bad row undoes everything, the table stays as it was;
   - **Skip and go on**: bad rows are left out and listed, the rest is committed.
7. Press the button at the bottom (“Import 5 columns”) and confirm “Import into *table*?” with **Import**.

- The import runs in a single transaction. **Cancel** while it runs writes no rows.
- At the end: “Imported 12,400 rows into customers in 3 s”, plus the **Rejected rows** with line number and reason (the first 500).
- On a production connection the dialog shows the write switch and the import asks for the confirmation chosen in Settings: see [Production safety](production).

## Import documents from a file
For MongoDB, Elasticsearch and DynamoDB: right-click a collection, an index or a table › **Import documents…**, or the "Import documents…" command in [[key:palette]].
1. **Choose a file…**: a JSON array, one document per line (NDJSON, JSON Lines, the output of `mongoexport`) or documents one after another, in `.json`, `.ndjson` or `.jsonl` files.
2. NiceDB reads the whole file and says how many documents it holds, shows the first twenty ([[←]] and [[→]] in the window) and lists the lines it cannot read.
3. Check the name of the collection, index or table: a new name creates the collection on MongoDB and the index on Elasticsearch; on DynamoDB the table must exist.
4. **Skip and go on** leaves out refused documents and lists them with their line; **Stop at the first error** stops there.
5. **Import N documents**. On a production connection it needs the unlock and the connection name, like any write.

Extended JSON (`$oid`, `$date`, `$numberLong`, `$numberDecimal`…) becomes the right type on MongoDB. There is no transaction: documents are written in blocks of 500 and the ones already written stay even if you stop or cancel. On MongoDB a block goes to the server in one go; on Elasticsearch and DynamoDB one document per request, slower. The import goes into the write log.

## From the command line
`nicedb run` runs a SQL file on a connection saved in NiceDB and writes the results as CSV, without opening the window. It uses the app's connections and the keychain's passwords: they are not written anywhere.

```
nicedb run --conn "Observatory — staging" report.sql > report.csv
echo "select count(*) from stars" | nicedb run --conn "Observatory — staging" --tsv -
```

| Option | Effect |
|---|---|
| `--conn` | the connection, by its name in NiceDB (or its id) |
| `--db` | another database of the same server |
| `--csv`, `--tsv` | comma-separated (the default) or tab-separated |
| `--confirm "name"` | on a production connection opens writes and confirms them with the connection's name |
| `--risky` | also runs risky statements (`DROP`, `TRUNCATE`, `DELETE` without `WHERE`…) |

Each statement that returns rows produces a block with its header, separated from the next by a blank line; rows touched and errors go to standard error. It exits with 0 when everything ran, 1 when a statement failed, 2 when the command was wrong. Writes go into the write log like the app's. On Windows the output shows when redirected to a file or another command.

An **Encrypted package** (`.nicedb`) carries folders, connections and, optionally, more, protected by a passphrase.

1. In the sidebar, **More actions** (the three dots) › **Export…** for the whole current workspace; or right-click a folder or a connection › **Export…**.
2. In **Format** choose **Encrypted package** or **JSON definitions**.
3. In **What to export** tick folders and connections.
4. For the encrypted package only, in **Along with**: the workspaces, **Query history**, **Scripts, snippets and notes**, **Appearance settings**, **Database passwords**.
5. Type the **Passphrase** (at least 8 characters) and **Repeat the passphrase**.
6. **Export…** and choose the file.

- Send the passphrase some other way. Without it the package does not open, and there is no way to recover it.
- **Database passwords** stay inside the encrypted package only: whoever knows the passphrase can read them. SSH, proxy and certificate passphrases never leave: whoever imports uses their own.
- **JSON definitions** carry folders and connections only, in plain text, without passwords or SSH secrets.

## Import a package
1. **More actions** › **Import…** and pick a `.nicedb` or `.json` file.
2. If the package is encrypted, type the **Passphrase** and press **Open**.
3. Check the list. New items are marked “new”; for those that already exist, choose what to do:

   | Item | Choices |
   |---|---|
   | Connection, workspace | **Keep both** (default), **Replace**, **Skip** |
   | Folder | **Merge** (default), **Separate** |
   | Appearance settings | **Keep mine** (default), **Replace** |

4. **Import**. The summary says how many items were added, replaced and left as they were.

Nothing is overwritten without a choice.

## Other data to export
- The write log exports to CSV or JSON from the **Log** panel of the Data view: see [Production safety](production).
- Database backups with the engine's own tools are in **Tools › Backup and restore**: see [Tools](tools).

## Limitations
- Palette pins and bookmarks and test-data profiles do not travel in packages.
- A package made by a newer NiceDB does not open in an older one.
- An Excel file is imported one sheet at a time.
