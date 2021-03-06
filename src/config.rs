use std::path::Path;
use std::fs::File;
use std::io::{Read, Write};
use clap::{App, Arg};
use crate::error::QanError;

/// Struct responsible for the on start defineable parameters.
#[derive(Serialize,Deserialize)]
pub struct Config{
    pub spv         : u64,
    pub root        : String,
    pub logging     : String,
    pub rpc_port    : u16,
    pub rpc_user    : String,
    pub rpc_pass    : String,
    pub rpc_auth    : String,
    pub bootstrap   : Vec<String>,
}

impl std::default::Default for Config{
    fn default() -> Self{
        Config{
            spv         : 0,
            root        : "./".into(),
            rpc_port    : 8000,
            rpc_user    : "unexpected".into(),
            rpc_pass    : "pacal".into(),
            rpc_auth    : "Basic dW5leHBlY3RlZDpwYWNhbA==".into(),
            bootstrap   : vec!("127.0.0.1:4222".into()),
            logging     : "".to_string(),
        }
    }
}

impl Config{
    pub fn from_string( s : &str ) -> Result<Self, QanError> {
        Ok(toml::from_str(s).unwrap())
    }

    pub fn to_string( &self ) -> Result<String, QanError> {
        Ok(toml::to_string(&self).unwrap())
    }

    pub fn get_config() -> Result<(Self, log4rs::Handle), QanError> {
        let mut config = if Path::new("./config.toml").exists(){
            let mut buf = String::new();
            File::open("./config.toml").map_err(|e|QanError::Io(e))?.read_to_string(&mut buf);
            Config::from_string(&buf)?
        }else{
            Config::default()
        };
        let matches = App::new("POA").args(&[
            Arg::with_name("rpc-user")
                .help("http authentication username")
                .takes_value(true)
                .short("u")
                .long("user"),
            Arg::with_name("rpc-pwd")
                .help("http authentication password")
                .takes_value(true)
                .short("k")
                .long("password"),
            Arg::with_name("rpc-port")
                .help("rpc port")
                .takes_value(true)
                .short("p")
                .long("port"),
            Arg::with_name("root")
                .help("nats server uri")
                .takes_value(true)
                .short("r")
                .long("root"),
            Arg::with_name("nats")
                .help("root directory")
                .takes_value(true)
                .short("n")
                .long("nats"),
            Arg::with_name("spv")
                .help("sync depth. 0 is full sync. ex. 50 means \"sync the top 50 blocks\"")
                .takes_value(true)
                .short("s")
                .long("spv"),
            Arg::with_name("logging")
                .help("set logging level")
                .takes_value(true)
                .short("l")
                .long("logging"),
        ]).get_matches();

    
        if let Some(u) = matches.value_of("rpc-user") {
            let mut token_base = String::new();
            token_base.push_str(u);
            config.rpc_user = u.into();
            token_base.push_str(":");
            if let Some(k) = matches.value_of("rpc-pwd") {
                token_base.push_str(k);
                config.rpc_pass = k.into();
            }
            config.rpc_auth = "Basic ".to_owned() + &base64::encode(&token_base);
        }
        if let Some(n) = matches.value_of("nats") { config.bootstrap = vec![n.to_owned()] }
        if let Some(r) = matches.value_of("root") { config.root = r.into() }
        if let Some(p) = matches.value_of("rpc-pwd") { config.rpc_port = p.parse::<u16>().expect("invalid port") }
        if let Some(s) = matches.value_of("spv") { config.spv =  s.parse::<u64>().expect("invalid sync depth") }
        if let Some(l) = matches.value_of("logging") { config.logging = l.into() }

        let log_handle = crate::util::init_logging(&config.logging);

        if !Path::new("./config.toml").exists(){
            File::create("./config.toml").map_err(|e|QanError::Io(e))?.write_fmt(format_args!("{}",config.to_string()?));
        }
        Ok((config, log_handle))
    }
}

