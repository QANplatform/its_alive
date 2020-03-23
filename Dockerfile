FROM rust:latest
LABEL version = "0.1" \
        description = "poa" \
        vendor = "cc"
SHELL ["/bin/bash", "-c"]
RUN apt-get update -y 
RUN apt-get install -y libclang-dev llvm clang build-essential
RUN useradd -m qand
#WORKDIR /tmp
#RUN git clone 'https://github.com/facebook/rocksdb.git'
#WORKDIR /tmp/rocksdb
#RUN make shared_lib
#RUN make install
WORKDIR /home/qand/PoaDemo/
COPY . .
RUN chown -R qand:qand .
USER qand
RUN cargo build
ENTRYPOINT ./target/debug/poa_demo -u=user -p=password -n=172.33.0.1:4222
EXPOSE 4222
EXPOSE 8000
