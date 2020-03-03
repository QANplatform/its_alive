use std::{
    thread, 
    io::{stdin, BufRead},
};
use natsclient::{Client, ClientOptions};
use crate::{
    event::Event,
    block::Block,
    transaction::Transaction
};

pub fn start_client(opts: ClientOptions, sndr : &std::sync::mpsc::SyncSender<Event>) -> Client{
    let mut client = Client::from_options(opts).expect("13:client from options builder");
    client.connect().expect("41:client  connect");

    let bsndr = sndr.clone();
    client.subscribe("block.propose", move |msg| {
        bsndr.send(Event::Block(msg.payload.to_owned()));
        Ok(())
    }).expect("block.propose");

    let txsndr = sndr.clone();
    client.subscribe("tx.broadcast", move |msg| {
        txsndr.send(Event::Transaction(msg.payload.to_owned()));
        Ok(())
    }).expect("tx.broadcast");

    let chsndr = sndr.clone();
    client.subscribe("chat", move |msg| {
        chsndr.send(Event::Chat(msg.payload.to_owned()));
        Ok(())
    }).expect("chat");

    let pksndr = sndr.clone();
    client.subscribe("PubKey", move |msg| {
        pksndr.send(Event::PubKey(msg.payload.to_owned()));
        Ok(())
    }).expect("PubKey");
    client
}

pub fn start_sync_sub(sndr : &std::sync::mpsc::SyncSender<Event>, client : &Client){
    let syncsndr = sndr.clone();
    client.subscribe("Synchronize", move |msg| {
        let rep = msg.reply_to.clone().unwrap();
        syncsndr.send(Event::Synchronize(msg.payload.to_owned(), rep));
        Ok(())
    }).expect("Synchronize");
}

pub fn start_stdin_handler(tsndr : &std::sync::mpsc::SyncSender<Event>){ 
    let tsndr = tsndr.clone();
    thread::spawn( move ||{
        let stdin = stdin();
        let mut handle = stdin.lock();
        loop{
            let mut buffer = String::new();
            handle.read_line(&mut buffer);
            tsndr.send(Event::Chat(buffer.as_bytes().to_vec()));
        }
    });
}