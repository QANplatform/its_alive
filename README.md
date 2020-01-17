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
