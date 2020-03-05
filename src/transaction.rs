use std::fmt;
// use rmps::{Serializer, Deserializer};
#[cfg(not(feature = "quantum"))]
use ed25519_dalek::{Keypair, PublicKey, Signature};
use rand::RngCore;
use rand::rngs::OsRng;
use crate::util::blake2b;
use hex::encode;
#[cfg(feature = "quantum")]
use glp::glp::{GlpSig, GlpSk, GlpPk, sign, verify, gen_pk};

//TxBody is the main data of the transactions
#[derive(Debug, PartialEq, Deserialize, Serialize, Eq, Hash, Clone)]
pub struct TxBody{
	pub nonce: u64,             //size: 8     byte
	pub timestamp: u64,         //size: 8     byte
    pub recipient: [u8; 32],    //size: 32    byte
    pub data: Vec<u8>,          //size: x   byte
}

impl fmt::Display for TxBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"nonce\":{},\n\"timestamp\":{},\n\"recipient\":{},\n\"data\":{:?}",
        self.nonce, self.timestamp, encode(self.recipient), self.data)
    }
}

impl TxBody{
    pub fn new(recipient: [u8; 32], data: Vec<u8>) -> TxBody {
        TxBody {
            nonce: OsRng.next_u64(),  
            timestamp: crate::util::timestamp(),
            recipient: recipient,
            data: data,
        }
    }

    pub fn hash(&self) -> [u8;32]{
        blake2b(&serde_json::to_vec(&self).unwrap())
    }

    pub fn len(&self) -> usize{
        48+self.data.len()
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Eq, Hash, Clone)]
pub struct Transaction {
    pub transaction : TxBody,
    #[cfg(not(feature = "quantum"))]
    pub pubkey      : [u8;32],
    #[cfg(feature = "quantum")]
    pub pubkey      : [u8; 32],
    pub sig         : Vec<u8> 
}


impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"transaction\":{{{}}},\n\"pubkey\":{},\n\"sig\":{:?}",
        self.transaction, encode(&self.pubkey), encode(&self.sig))
    }
}


impl Transaction{
    #[cfg(not(feature = "quantum"))]
    pub fn new( transaction: TxBody, kp: &Keypair ) -> Transaction {
        let sig = kp.sign(&serde_json::to_vec(&transaction).unwrap());
        Transaction { transaction , pubkey: blake2b(&kp.public.to_bytes().to_vec()), sig: sig.to_bytes().to_vec() }
    }

    #[cfg(not(feature = "quantum"))]
    pub fn verify(&self, pubkey : &PublicKey) -> bool{
        let sig = Signature::from_bytes(&self.sig).unwrap();
        match pubkey.verify(&serde_json::to_vec(&self.transaction).unwrap(), &sig){
            Ok(_)=>true,
            Err(_)=>false
        }
    }

    #[cfg(feature = "quantum")]
    pub fn new( transaction: TxBody, sk: &GlpSk ) -> Transaction {
        let sig = sign(&sk, serde_json::to_vec(&transaction).unwrap()).unwrap();
        Transaction { transaction , pubkey: blake2b(&gen_pk(&sk).to_bytes()), sig: sig.to_bytes() }
    }

    #[cfg(feature = "quantum")]
    pub fn verify(&self, pubkey : &GlpPk) -> bool{
        let qsig = GlpSig::from_bytes(&self.sig);
        verify(&pubkey,&qsig,&serde_json::to_vec(&self.transaction).unwrap())
    }

    pub fn hash(&self) -> [u8;32]{
        blake2b(&serde_json::to_vec(&self).unwrap())
    }

    pub fn len(&self) -> usize{
        self.sig.len()+self.transaction.len()
    }
}