use std::{
    thread, 
    io::{stdin, BufRead},
};
use natsclient::{Client, ClientOptions};
use crate::{
    event::Event,
    block::Block,
    error::QanError,
    transaction::Transaction
};

pub fn start_client(opts: ClientOptions, sndr : &std::sync::mpsc::SyncSender<Event>) -> Result<Client,QanError>{
    let mut client = Client::from_options(opts).map_err(|e|QanError::Nats(e))?;
    client.connect().map_err(|e|QanError::Nats(e))?;

    let bsndr = sndr.clone();
    client.subscribe("block.propose", move |msg| {
        bsndr.send(Event::Block(msg.payload.to_owned()));
        Ok(())
    }).map_err(|e|QanError::Nats(e))?;

    let txsndr = sndr.clone();
    client.subscribe("tx.broadcast", move |msg| {
        txsndr.send(Event::Transaction(msg.payload.to_owned()));
        Ok(())
    }).map_err(|e|QanError::Nats(e))?;

    let pksndr = sndr.clone();
    client.subscribe("PubKey", move |msg| {
        pksndr.send(Event::PubKey(msg.payload.to_owned(), msg.reply_to.clone()));
        Ok(())
    }).map_err(|e|QanError::Nats(e))?;
    Ok(client)
}

pub fn start_sync_sub(sndr : &std::sync::mpsc::SyncSender<Event>, client : &Client) -> Result<(), QanError>{
    let syncsndr = sndr.clone();
    client.subscribe("Synchronize", move |msg| {
        let rep = msg.reply_to.clone().unwrap();
        syncsndr.send(Event::Synchronize(msg.payload.to_owned(), rep));
        Ok(())
    }).map_err(|e|QanError::Nats(e))?;
    Ok(())
}

pub fn start_stdin_handler(tsndr : &std::sync::mpsc::SyncSender<Event>){ 
    let tsndr = tsndr.clone();
    thread::spawn( move ||{
        let stdin = stdin();
        let mut handle = stdin.lock();
        loop{
            let mut buffer = String::new();
            handle.read_line(&mut buffer);
            // tsndr.send(Event::Chat(buffer.as_bytes().to_vec()));
        }
    });
}