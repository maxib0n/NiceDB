# Importare ed esportare

## Esportare una tabella
1. Tasto destro sulla tabella nell'esploratore › **Esporta dati…**. In alternativa: **Strumenti › Importa ed esporta**, scegli la tabella e premi **Esporta…**; oppure il comando «Esporta *tabella* in…» della palette, che chiede subito il formato.
2. Scegli il **Formato**:

   | Formato | Per cosa |
   |---|---|
   | CSV | virgole, per ogni programma |
   | TSV | tabulazioni, da incollare |
   | JSON | numeri e JSON con il loro tipo |
   | XLSX | un foglio, numeri come numeri |
   | Parquet | colonnare e compresso |

3. Per CSV, TSV e XLSX lascia o togli **Prima riga con i nomi delle colonne**.
4. **Esporta…** e scegli dove salvare il file.

- Il file si scrive mentre la tabella si legge: non c'è limite di righe in memoria.
- La barra mostra l'avanzamento; **Annulla** ferma tutto ed elimina il file parziale.
- Alla fine: righe scritte, nome del file, dimensione e tempo.
- Un foglio Excel tiene al massimo 1.048.576 righe: oltre, usa CSV o Parquet.

## Esportare il risultato di una query
1. Esegui la query.
2. **Esporta** sopra i risultati, oppure tasto destro sulla scheda dei risultati › **Esporta**.
3. Scegli formato e file come per una tabella.

La query viene rieseguita su una connessione a parte e scritta mentre arriva, quindi il file contiene tutte le righe, non solo quelle nella griglia. Si esporta solo il risultato di una `SELECT`.

Per poche righe basta la griglia: tasto destro › **Copia come** › **Testo con intestazioni**, **Come CSV**, **Come JSON** o **Come INSERT**. Vedi [Eseguire e leggere i risultati](results).

## Importare un file in una tabella
1. Apri l'importazione:
   - tasto destro su una tabella › **Importa dati…**, per importare in quella tabella;
   - **Strumenti › Importa ed esporta › Importa un file…**;
   - il comando «Importa dati in *tabella*…» della palette.
2. **Scegli un file…**: CSV e TSV (separatore rilevato, anche punto e virgola), JSON (un array di oggetti o un oggetto per riga), XLSX, Parquet. Il file non viene caricato tutto in memoria.
3. Controlla l'anteprima. Se serve, cambia **Prima riga con i nomi** e, per un file Excel, il **Foglio**.
4. Scegli la **Destinazione**:
   - **Tabella esistente**: in **Corrispondenza delle colonne** ogni colonna della tabella prende una colonna del file, abbinata per nome quando si può; «— lascia vuota» la salta.
   - **Nuova tabella**: scrivi il **Nome della tabella**; per ogni colonna del file scegli se importarla, il **Nome della colonna** e il **Tipo** proposto.
5. Lascia o togli **Celle vuote come NULL**.
6. Scegli cosa fare delle **Righe con errori**:
   - **Fermati al primo errore**: una riga sbagliata annulla tutto, la tabella resta com'era;
   - **Salta e prosegui**: le righe sbagliate restano fuori e vengono elencate, il resto viene confermato.
7. Premi il pulsante in basso («Importa 5 colonne») e conferma «Importare in *tabella*?» con **Importa**.

- L'importazione avviene in una sola transazione. **Annulla** durante il lavoro non scrive nessuna riga.
- Alla fine: «Importate 12.400 righe in clienti in 3 s», più le **Righe scartate** con numero di riga e motivo (le prime 500).
- Su una connessione di produzione la finestra mostra l'interruttore della scrittura e l'importazione chiede la conferma scelta in Impostazioni: vedi [Sicurezza in produzione](production).

## Importare documenti da un file
Per MongoDB, Elasticsearch e DynamoDB: tasto destro su una collezione, un indice o una tabella › **Importa documenti…**, oppure il comando «Importa documenti…» in [[key:palette]].
1. **Scegli un file…**: un array JSON, un documento per riga (NDJSON, JSON Lines, l'uscita di `mongoexport`) o documenti uno dopo l'altro, in file `.json`, `.ndjson` o `.jsonl`.
2. NiceDB legge tutto il file e dice quanti documenti contiene, mostra i primi venti (con [[←]] e [[→]] nella finestra) ed elenca le righe che non si leggono.
3. Controlla il nome della collezione, dell'indice o della tabella: un nome nuovo crea la collezione su MongoDB e l'indice su Elasticsearch; su DynamoDB la tabella deve esistere.
4. **Salta e prosegui** lascia fuori i documenti rifiutati e li elenca con la riga; **Fermati al primo errore** si ferma lì.
5. **Importa N documenti**. Su una connessione di produzione serve lo sblocco e il nome della connessione, come per ogni scrittura.

Il JSON esteso (`$oid`, `$date`, `$numberLong`, `$numberDecimal`…) su MongoDB diventa il tipo giusto. Non c'è transazione: i documenti si scrivono a blocchi di 500 e quelli già scritti restano anche se fermi o annulli. Su MongoDB un blocco va al server in una volta sola; su Elasticsearch e DynamoDB un documento per richiesta, più lento. L'importazione finisce nel registro delle scritture.

## Dalla riga di comando
`nicedb run` esegue un file SQL su una connessione salvata in NiceDB e scrive i risultati in CSV, senza aprire la finestra. Usa le connessioni dell'app e le password del portachiavi: non vanno scritte da nessuna parte.

```
nicedb run --conn "Bassi — staging" report.sql > report.csv
echo "select count(*) from ordini" | nicedb run --conn "Bassi — staging" --tsv -
```

| Opzione | Effetto |
|---|---|
| `--conn` | la connessione, col nome che ha in NiceDB (o il suo id) |
| `--db` | un altro database dello stesso server |
| `--csv`, `--tsv` | separati da virgola (di serie) o da tabulazione |
| `--confirm "nome"` | su una connessione di produzione apre la scrittura e la conferma col nome della connessione |
| `--risky` | esegue anche le istruzioni rischiose (`DROP`, `TRUNCATE`, `DELETE` senza `WHERE`…) |

Ogni istruzione che restituisce righe produce un blocco con l'intestazione, separato dal successivo da una riga vuota; righe toccate ed errori vanno sull'uscita degli errori. Termina con 0 se tutto è andato, 1 se un'istruzione è fallita, 2 se il comando è sbagliato. Le scritture finiscono nel registro come quelle dell'app. Su Windows l'uscita si vede reindirizzandola in un file o in un altro comando.

Un **Pacchetto cifrato** (`.nicedb`) porta cartelle, connessioni e, a scelta, altro, protetto da una frase segreta.

1. Nella barra laterale, **Altre azioni** (i tre puntini) › **Esporta…** per tutto lo spazio di lavoro attuale; oppure tasto destro su una cartella o una connessione › **Esporta…**.
2. In **Formato** scegli **Pacchetto cifrato** o **Definizioni JSON**.
3. In **Cosa esportare** spunta cartelle e connessioni.
4. Solo per il pacchetto cifrato, in **Insieme**: gli spazi di lavoro, **La cronologia delle query**, **Gli script, gli snippet e le note**, **Le impostazioni di aspetto**, **Le password dei database**.
5. Scrivi la **Frase segreta** (almeno 8 caratteri) e **Ripeti la frase**.
6. **Esporta…** e scegli il file.

- Manda la frase segreta per un'altra strada. Senza la frase il pacchetto non si apre e non c'è modo di recuperarla.
- **Le password dei database** restano solo dentro il pacchetto cifrato: chi conosce la frase può leggerle. Le passphrase di SSH, proxy e certificati non partono mai: chi importa usa le sue.
- **Definizioni JSON** porta solo cartelle e connessioni, in chiaro, senza password né segreti SSH.

## Importare un pacchetto
1. **Altre azioni** › **Importa…** e scegli un file `.nicedb` o `.json`.
2. Se il pacchetto è cifrato, scrivi la **Frase segreta** e premi **Apri**.
3. Controlla l'elenco. Le voci nuove sono segnate «nuova»; per quelle che esistono già scegli cosa fare:

   | Voce | Scelte |
   |---|---|
   | Connessione, spazio di lavoro | **Tieni entrambe** (di serie), **Sostituisci**, **Salta** |
   | Cartella | **Unisci** (di serie), **Separata** |
   | Impostazioni di aspetto | **Tieni le mie** (di serie), **Sostituisci** |

4. **Importa**. Il riepilogo dice quante voci sono state aggiunte, sostituite e lasciate com'erano.

Nulla viene sovrascritto senza una scelta.

## Altri dati da esportare
- Il registro delle scritture si esporta in CSV o JSON dal pannello **Registro** della vista Dati: vedi [Sicurezza in produzione](production).
- Il backup di un database con gli strumenti del motore è in **Strumenti › Backup e ripristino**: vedi [Strumenti](tools).

## Limiti
- Fissati e segnalibri della palette e i profili dei dati di prova non viaggiano nei pacchetti.
- Un pacchetto fatto con una versione più recente di NiceDB non si apre con una più vecchia.
- Un file Excel si importa un foglio alla volta.
