# Rchat
----
Rchat is a encrypted, private and memory safe remote chat, built in rust. It is modular and supports features such as sending files.
# Install
---
```
sudo apt update &&
sudo apt upgrade &&
sudo apt install git &&
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh &&
git clone https://github.com/H4k1l/Rchat.git &&
cd Rchat &&
cargo run -- -h
```
# Screenshots
----
![Rchat](https://github.com/H4k1l/Rchat/blob/main/images/screenshot1.png)
# Usage
----
```
cargo run -- -h
Rchat is a encrypted, private and memory safe remote chat, built in rust

Usage: Rchat [OPTIONS] --port <PORT> --name <NAME>

Options:
  -a, --address <ADDRESS>     the address to connect to
  -p, --port <PORT>           the port to host/connect
      --host                  host a connection
      --connect               connect to an host
  -n, --name <NAME>           your name identifier
      --protected <PASSWORD>  the password to be protected be
  -h, --help                  Print help
```
For host a chat:
```
cargo run -- --host --port <PORT> --name <NAME>
```
For connect to a remote host:
```
cargo run -- --connect -a <ADDRESS> --port <PORT> --name <NAME>
```
For clear the chat while in chat:
```
!clear
```
# !!!Disclaimers!!!
----
!!Rchat is currently in developing and is not finished yet, many parts of the software is not avalable yet!!
The author is not responsible for any damages, misuse or illegal activities resulting from the use of this code.
