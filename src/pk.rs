use std::{
    io::{Read, Write},
    fs::File,
    path::Path
};
#[cfg(feature = "quantum")]
use glp::glp::{GlpSk, gen_sk, GlpPk, gen_pk};
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;

#[cfg(not(feature = "quantum"))]
pub const PATHNAME  : &'static str = "./SECURE_THIS.pem";

#[cfg(feature = "quantum")]
pub const PATHNAME  : &'static str = "./qSECURE_THIS.pem";

#[cfg(feature = "quantum")]
const DELIMITER : &'static str = "---PETPRIVATEKEYDELIMITER---";
const EC_PK_SIZE: usize = 64;
#[cfg(feature = "quantum")]
const Q_PK_SIZE : usize = 4096;

pub struct PetKey {
    #[cfg(feature = "quantum")]
    pub glp: GlpSk,
    pub ec:  Keypair,  
}

impl PetKey {
        #[cfg(not(feature = "quantum"))]
    pub fn new() -> Self{
        PetKey{
            ec  : Keypair::generate(&mut OsRng)
        }
    }

    #[cfg(feature = "quantum")]
    pub fn new() -> Self{
        PetKey{
            glp : gen_sk(),
            ec  : Keypair::generate(&mut OsRng)
        }
    }

    #[cfg(feature = "quantum")]
    pub fn get_glp_pk(&self) -> GlpPk{
        gen_pk(&self.glp)
    }

    #[cfg(feature = "quantum")]
    pub fn get_glp_pk_bytes(&self) -> Vec<u8>{
        gen_pk(&self.glp).to_bytes()
    }

    #[cfg(not(feature = "quantum"))]
    pub fn new_from_keys( ec: ed25519_dalek::Keypair) -> Self {
        PetKey{ ec }
    }

    #[cfg(feature = "quantum")]
    pub fn new_from_keys(glp: GlpSk, ec: ed25519_dalek::Keypair) -> Self {
        PetKey{ glp , ec }
    }

    #[cfg(not(feature = "quantum"))]
    pub fn to_bytes(&self)->Vec<u8>{
        self.ec.to_bytes().to_vec()
    }

    #[cfg(feature = "quantum")]
    pub fn to_bytes(&self)->Vec<u8>{
        let mut ret : Vec<u8> = Vec::new();
        let mut glpb = self.glp.to_bytes();
        let mut delimiterb = DELIMITER.as_bytes().to_vec();
        let mut ecb = self.ec.to_bytes().to_vec();
        ret.append(&mut glpb);
        ret.append(&mut delimiterb);
        ret.append(&mut ecb);
        ret
    }

    #[cfg(not(feature = "quantum"))]
    pub fn from_bytes(b : &Vec<u8>) -> Self{
        let mut ec  : [u8;EC_PK_SIZE]= [0;EC_PK_SIZE];
        ec.copy_from_slice(&b[..]);
        PetKey{
            ec  : Keypair::from_bytes(&ec).unwrap()
        }
    }

    #[cfg(feature = "quantum")]
    pub fn from_bytes(b : &Vec<u8>) -> Self{
        let mut glp : [u8;Q_PK_SIZE] = [0;Q_PK_SIZE];
        let mut ec  : [u8;EC_PK_SIZE]= [0;EC_PK_SIZE];
        
        glp.copy_from_slice(&b[0..Q_PK_SIZE]);
        ec.copy_from_slice(&b[(Q_PK_SIZE+DELIMITER.len())..]);

        PetKey{
            glp : GlpSk::from_bytes(&glp.to_vec()),
            ec  : Keypair::from_bytes(&ec).unwrap()
        }
    }

    pub fn write_pem(&self){
        let mut pemf = File::create(Path::new(PATHNAME)).unwrap();
        pemf.write_all(&self.to_bytes());
    }

    pub fn from_pem(pathname : &str) -> Self{
        let mut pemf = File::open(Path::new(pathname)).unwrap();
        let mut buffer = Vec::new();
        pemf.read_to_end(&mut buffer);
        Self::from_bytes(&buffer)
    }
}

#[test]
fn to_from_pem() {
    let keys = PetKey::new();
    println!("{}", keys.to_bytes().len());
    keys.write_pem();
    let keys2= PetKey::from_pem();
    assert_eq!(1, 2); 
}