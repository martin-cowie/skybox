# skybox
A Rust crate and command line to interact with Sky Plus hardware.

Taking [this project](https://github.com/martin-cowie/sky-box) as inspiration.

## Installation

```
cargo build && cargo install --path .
```

## Use

Scan for any Sky Plus hardware using `skybox scan`, e.g.

```
⠦ Scanning...
Found http://192.168.2.152:49153/...SkyPlay and http://192.168.2.15
   Found 1 skybox
0:      Some(Ipv4(192.168.2.152))
Choose a skybox:
```

Dump your recordings as CSV using `skybox ls`

Remove one or more recordings using `skybox rm` e.g.
```
skybox rm BOOK:688614341 BOOK:688614366
```

Play a recordin using `skybox play` e.g.

```
skybox play file://pvr/290AFCC5
```