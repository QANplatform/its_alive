FROM rustlang/rust:nightly
LABEL version = "0.1" \
        description = "asd" \
        vendor = "cc"
SHELL ["/bin/bash", "-c"]
#RUN apt update -y && apt-get install -y --fix-missing gcc libgmp-dev build-essential automake m4 wget telnet nmap curl openssl libssl1.1 libssl-dev libevent-dev libleveldb-dev
WORKDIR /home/qanprivate/
COPY * /home/qanprivate/
RUN cargo build
EXPOSE 8000
ENTRYPOINT ./target/debug/qanprivate