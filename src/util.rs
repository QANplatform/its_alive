use std::io::prelude::*;
use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};
use blake2::{Blake2b, /*Blake2s,*/ Digest as BDigest};
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::Handle;

/// Getter function for unix timestamp in milliseconds
pub fn timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("timestamp").as_millis() as u64
}

/// Reader function to get n bytes from /dev/urandom
pub fn urandom(n: usize) -> Vec<u8> {
    let mut fd = File::open("/dev/urandom").expect("failed to open urandom");
    let mut ret = vec![0; n];
    fd.read(&mut ret).expect("failed to read urandom");
    // println!("urandom in new line:\n {:?}", ret);
    ret.to_vec()
}

/// Convinience function to handle the feature flags at hashing
pub fn do_hash(input: &Vec<u8>) -> [u8;32]{
    #[cfg(feature = "quantum")]
    let ret = swift_hash(input);
    #[cfg(not(feature = "quantum"))]
    let ret =  blake2b(input);
    ret
}

/// Function to handle to quantum secure hashing
/// # Safety 
/// this function calls out into C a codebase
#[cfg(feature = "quantum")]
pub fn swift_hash(input: &Vec<u8>) -> [u8;32]{
    let mut ret = [0u8;32];
    unsafe{
        crate::hash::SWIFFTX(256, input.as_ptr(), input.len(), ret.as_mut_ptr());
    }
    ret
}

/// Usage function for blake2b hashing
pub fn blake2b(b: &Vec<u8>) ->  [u8;32] {
    let mut hasher = Blake2b::new();
    hasher.input(b);
    let result = hasher.result();
    let mut ret = [0u8;32];
    for i in 0..32 {
        ret[i] = result[i];
    }
    ret
}

/// Convinience function
pub fn vec_to_arr(v : &Vec<u8>)->[u8;32]{
    let mut a : [u8;32] =[0;32];
    a.clone_from_slice(&v[0..32]);
    a
}

/// Init function for logging. Parameter decides the level of logging. The log file is placed at `/tmp/qan/` named `<unix timestamp of starting>.log` 
pub fn init_logging(level_conf : &str) -> log4rs::Handle{
    let level = match level_conf.as_ref(){
        "error" =>LevelFilter::Error,
        "warn"  =>LevelFilter::Warn,
        "info"  =>LevelFilter::Info,
        "debug" =>LevelFilter::Debug,
        "trace" =>LevelFilter::Trace,
        _       =>LevelFilter::Error,
    };

    let logname = "/tmp/qan/".to_owned()+&timestamp().to_string()+".log";
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S %Z)(utc)} - {m}{n}")))
        .build(logname)
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .logger(Logger::builder().appender("logfile").additive(true).build("logfile", level))
        .build(Root::builder().appender("logfile").build(level))
        .unwrap();

    log4rs::init_config(config).unwrap()
}