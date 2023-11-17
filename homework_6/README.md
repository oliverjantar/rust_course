# Chat client & server

## Contents
1. [Example Usage](#example-usage)
2. [Server](#server)
3. [Client](#client)

## Example usage

To run the chat server and clients, use commands: 
```
$ cargo run --bin server
$ cargo run --bin client 
```
When starting client, it is recommended to use argument `--username` to know who is connecting to the server. To explore all options when running client and server, see [server](#server) and [client](#client) sections.
```
$ cargo run --bin client -- --username lukas
```
Once you run the server instance, you can start the clients. The typical output of the clients is:

#### Client 1 output
```
$ cargo run --bin client -- --username lukas
```
```
Connecting to server on 127.0.0.1:11111...
Connected. You can now send messages.
--      Active users: 0      --
--      New user connected: tomas      --
tomas: Ahoj
Cau
tomas: posilam ti soubor, checkni to
tomas sent a file input.txt
File saved to: ./data/files/input.txt
Mam to, husty!

```

#### Client 2 output
```
$ cargo run --bin client -- --username tomas
```
```
Connecting to server on 127.0.0.1:11111...
Connected. You can now send messages.
--      Active users: 1      --
Ahoj
lukas: Cau
posilam ti soubor, checkni to
.file ./data/input.txt
lukas: Mam to, husty!
```

## Server

When server is started, debug tracing logs are sent to the standard output.

```
Options:
      --host <HOST>  Server Host [default: 127.0.0.1]
  -p, --port <PORT>  Server Port [default: 11111]
  -h, --help         Print help
```

## Client
When client is started, debug tracing logs are saved to ./logs directory. The output can be changed with argument `--logs-dir`.

Client can be started with these options:

```
Options:
      --host <HOST>              Server Host [default: 127.0.0.1]
  -p, --port <PORT>              Server Port [default: 11111]
  -o, --output-dir <OUTPUT_DIR>  Directory to save incoming files and images [default: ./data]
  -l, --logs-dir <LOGS_DIR>      Directory to save tracing logs from client [default: ./logs]
  -u, --username <USERNAME>      Username [default: anonymous]
  -h, --help                     Print help
  ```

