## PoA RAFT stack test

```ascii
 ________  ________  ________
|\   __  \|\   __  \|\   ___  \
\ \  \|\  \ \  \|\  \ \  \\ \  \
 \ \  \\\  \ \   __  \ \  \\ \  \
  \ \  \\\  \ \  \ \  \ \  \\ \  \
   \ \_____  \ \__\ \__\ \__\\ \__\
    \|___| \__\|__|\|__|\|__| \|__|
          \|__|
```

This repository is a premature effort to glue together our
individual modules which includes:
- NATS-based synchronous network
- DB layer and serialization
- Naive transaction validation

Note that our Quantum Cryptographic stack dependency __fence__  is
deliberatly missing from the repository, as it's currently only
used in private investor demos and setups during the QAN IEO.

Have fun playing with QAN poa-0.0.1 daemon \o/

## Usage
### DISCLAIMER
This demo is a proof of concept, it does not represent the final product.
Thus far only linux distros have been tested.
The demo is built in a syncronous way.
Since rocksdb is used it does not allow the code to run multiple times 
from the same root folder.
Two alternatives are presented at "Running multiple instances".
### Dependencies
To run the demo some dependencies are needed beside the repo itself.
These dependencies can be read out of the Dockerfile,
but for convinience sake these are:
- rust 1.40 or newer
- clang, llvm and libclang-dev:
      `apt-get install -y libclang-dev llvm clang`
      (or equivalent, depending on distro)
- NATS server (working binary provided in the repo)
- rocksdb https://github.com/facebook/rocksdb.git 

### Building
When all dependencies are present the demo can be built by:
`cargo build`
or:
`cargo build --release`
for a smaller binary.
Using the "quantum" feature flag is not possible as not all sources are
presented as of yet. The flagged code pieces are present to provide
the actual working glue examples. 

### Running
The first running must be the NATS server as the node uses it. 
In case the NATS server is not running the demo itself will panic and exit.
A working NATS server binary is present at (from the repos root folder).
- `./nats-server-v2.1.2-linux-386/nats-server`
- `cargo run` or `cargo run --release`

Alternatively:
- `cargo build`
- `target/debug/poa_demo`

You can also use `-u` and `-p` to set http basic auth for the rpc.
A `-n` argument is also present to define NATS server location.
(default NATS uri: `nats://127.0.0.1:4222`)

The demo takes data from terminal and uses them to create transactions,
that the whole network receives.

The demo is also reachable by JSON-RPC on port `8000`.
Working jsons are presented in a separate `JSON_API.md` file as curl commands.

### Running multiple instances
RocksDB prevents us from having multiple instances use the same database.
One possibility to circumvent this is placing the compiled binary
in multiple folders, thus each instance creating and managing its own database.

The other is using docker.
The repo itself contains the `Dockerfile` needed, as well as a `compose.sh`.

Using the docker path has the following steps in order:
- `systemctl start docker`
- `docker network create subs` 
(the `subs` docker network is used in our `compose.sh`)
- `docker build . -t poademo`
- start NATS server on the **host machine**: `./nats-server-v2.1.2-linux-386/nats-server`
- `./compose.sh <number of nodes>` or
- `docker run -i --net=subs --name="node" -h "node" -d --ip="172.33.0.2" poademo`
