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

    pub fn serialize(&self)->String{
        serde_json::to_string(&self).unwrap()
    }

    pub fn hash(&self) -> [u8;32]{
        blake2b(&self.serialize().as_bytes().to_vec())
    }

    pub fn deserialize(v : &str)-> Transaction{
        serde_json::from_str(&v).unwrap()
    }

    pub fn deserialize_slice( s :&[u8] ) -> Self {
        serde_json::from_slice(s).unwrap()
    }

    pub fn len(&self) -> usize{
        36+self.data.len()
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Eq, Hash, Clone)]
pub struct Transaction {
    pub transaction : TxBody,
    #[cfg(not(feature = "quantum"))]
    pub pubkey      : [u8;32],
    #[cfg(feature = "quantum")]
    pub pubkey      : Vec<u8>,
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
        let sig = kp.sign(&transaction.serialize().as_bytes().to_vec());
        Transaction { transaction , pubkey: kp.public.clone().to_bytes(), sig: sig.to_bytes().to_vec() }
    }

    #[cfg(not(feature = "quantum"))]
    pub fn validate(&self) -> bool{
        let pk = PublicKey::from_bytes(&self.pubkey).unwrap();
        let sig = Signature::from_bytes(&self.sig).unwrap();
        match pk.verify(self.transaction.serialize().as_bytes(), &sig){
            Ok(_)=>true,
            Err(_)=>false
        }
    }

    #[cfg(feature = "quantum")]
    pub fn new( transaction: TxBody, sk: &GlpSk ) -> Transaction {
        let sig = sign(&sk, transaction.serialize().as_bytes().to_vec()).unwrap();
        Transaction { transaction , pubkey: gen_pk(&sk).to_bytes(), sig: sig.to_bytes() }
    }

    #[cfg(feature = "quantum")]
    pub fn validate(&self) -> bool{
        let pk = GlpPk::from_bytes(&self.pubkey);
        let qsig = GlpSig::from_bytes(&self.sig);
        verify(&pk,qsig,self.transaction.serialize().as_bytes().to_vec())
        // let ecsig = self.transaction.ec_sig;
        
    }


    pub fn serialize(&self)->String{
        serde_json::to_string(&self).unwrap()
    }

    pub fn hash(&self) -> [u8;32]{
        blake2b(&self.serialize().as_bytes().to_vec())
    }

    pub fn deserialize(v : &str)-> Transaction{
        serde_json::from_str(&v).unwrap()
    }

    pub fn deserialize_slice( s :&[u8] ) -> Self {
        serde_json::from_slice(s).unwrap()
    }

    pub fn len(&self) -> usize{
        self.sig.len()+self.transaction.len()
    }
}