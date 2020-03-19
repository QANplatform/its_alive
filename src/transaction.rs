use std::fmt;
// use rmps::{Serializer, Deserializer};
#[cfg(not(feature = "quantum"))]
use ed25519_dalek::{Keypair, PublicKey, Signature};
use rand::RngCore;
use rand::rngs::OsRng;
use crate::util::do_hash;
use crate::error::QanError;
use hex::encode;
#[cfg(feature = "quantum")]
use glp::glp::{GlpSig, GlpSk, GlpPk, sign, verify, gen_pk};
use jsonrpc_http_server::jsonrpc_core::Value;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VmCall{
    pub prev_root: [u8; 32],
    pub res_root : [u8; 32],
    pub sc_hash  : [u8; 32],
    pub func     : String,
    pub params   : Vec<Value>,
}

impl VmCall{
    pub fn len(&self) -> usize{
        96+self.func.len()+(self.params.len()*64)
    }
}

//TxBody is the main data of the transactions
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TxBody{
	pub nonce: u64,             //size: 8     byte
	pub timestamp: u64,         //size: 8     byte
    pub recipient: [u8; 32],    //size: 32    byte
    pub balance  : u64,         //size: 8     byte
    pub data: Option<VmCall>,
}

impl fmt::Display for TxBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"nonce\":{},\n\"timestamp\":{},\n\"recipient\":{},\n\"balance\":{}\n\"data\":{:?}",
        self.nonce, self.timestamp, encode(self.recipient), self.balance, self.data)
    }
}

impl TxBody{
    pub fn new(recipient: [u8; 32], balance: u64, data: Option<VmCall>) -> TxBody {
        TxBody {
            nonce: OsRng.next_u64(),  
            timestamp: crate::util::timestamp(),
            recipient,
            balance,
            data,
        }
    }

    pub fn hash(&self) -> Result<[u8;32],QanError>{
        Ok(do_hash(&serde_json::to_vec(&self).map_err(|e|QanError::Serde(e))?))
    }

    pub fn len(&self) -> usize{
        56 + match &self.data{
            Some(x)=> x.len(),
            None=>0
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Transaction {
    pub transaction : TxBody,
    pub pubkey      : [u8; 32],
    pub sig         : Vec<u8>,
}


impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"transaction\":{{{}}},\n\"pubkey\":{},\n\"sig\":{:?}",
        self.transaction, encode(&self.pubkey), encode(&self.sig))
    }
}


impl Transaction{
    #[cfg(not(feature = "quantum"))]
    pub fn new( transaction: TxBody, kp: &Keypair ) -> Result<Self, QanError> {
        let sig = kp.sign(&serde_json::to_vec(&transaction).map_err(|e|QanError::Serde(e))?);
        Ok(Transaction { transaction , pubkey: do_hash(&kp.public.to_bytes().to_vec()), sig: sig.to_bytes().to_vec() })
    }

    #[cfg(not(feature = "quantum"))]
    pub fn verify(&self, pubkey : &PublicKey) -> Result<bool, QanError>{
        let sig = Signature::from_bytes(&self.sig).unwrap();
        Ok(match pubkey.verify(&serde_json::to_vec(&self.transaction).map_err(|e|QanError::Serde(e))?, &sig){
            Ok(_)=>true,
            Err(_)=>false
        })
    }

    #[cfg(feature = "quantum")]
    pub fn new( transaction: TxBody, sk: &GlpSk ) -> Result<Self, QanError> {
        let sig = sign(&sk, serde_json::to_vec(&transaction).map_err(|e|QanError::Serde(e))?).unwrap();
        Ok(Transaction { transaction , pubkey: do_hash(&gen_pk(&sk).to_bytes()), sig: sig.to_bytes() })
    }

    #[cfg(feature = "quantum")]
    pub fn verify(&self, pubkey : &GlpPk) -> Result<bool, QanError>{
        let qsig = GlpSig::from_bytes(&self.sig);
        Ok(verify(&pubkey,&qsig,&serde_json::to_vec(&self.transaction).map_err(|e|QanError::Serde(e))?))
    }

    pub fn get_sc_call(&self) -> Option<VmCall>{
        self.transaction.data.clone()
    }

    pub fn hash(&self) -> Result<[u8;32], QanError>{
        Ok(do_hash(&serde_json::to_vec(&self).map_err(|e|QanError::Serde(e))?))
    }

    pub fn len(&self) -> usize{
        self.sig.len()+self.transaction.len()
    }
}