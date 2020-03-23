use serde::{Serialize, Deserialize};
use rmps::{Serializer, Deserializer};
use crate::error::QanError;
use crate::util::do_hash;
#[cfg(not(feature = "quantum"))]
use ed25519_dalek::{Keypair, PublicKey, Signature};
#[cfg(feature = "quantum")]
use glp::glp::{GlpSig, GlpSk, GlpPk, sign, verify, gen_pk};
use hex::encode;
use std::fmt;

/// Function used when making merkle-tree.
pub fn merge(l:&[u8;32],r:&[u8;32])->[u8;32]{
    let mut buf = Vec::new();
    buf.extend_from_slice(l);
    buf.extend_from_slice(r);
    do_hash(&buf) 
}

#[derive(Debug, Deserialize, Serialize,  Clone)]
pub struct BlockData {
    pub timestamp   : u64,
    pub merkle_root : Vec<u8>,
    pub prev_hash   : [u8;32],
    pub txes        : Vec<[u8;32]>,
}

impl fmt::Display for BlockData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tx_bases = Vec::new();
        for i in &self.txes{
            tx_bases.push(encode(&i));
        } 
        write!(f, "timestamp : {}\nmerkle_root : {}\nprev_hash : {:?}\ntxes : {:?}",
        self.timestamp, encode(&self.merkle_root), self.prev_hash, tx_bases)
    }
}

impl BlockData {
    /// Constructor function for BlockData. Takes a vector of the transaction hashes, and the hash of the previous block.
    pub fn new(prev_hash : [u8;32], txes : Vec<[u8;32]>) -> Result<Self, QanError> {
        let tree = static_merkle_tree::Tree::from_hashes(txes.to_vec(),merge);
        let merkle_root : Vec<u8> = tree.get_root_hash().unwrap().to_vec();
        Ok(BlockData{
            prev_hash,
            timestamp: crate::util::timestamp(),
            merkle_root,
            txes,
        })
    }

    /// getter for block timestamp
    pub fn timestamp(&self) -> u64{
        self.timestamp
    }

}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HashedBlock {
    pub blockdata   : BlockData,
    pub hash        : [u8;32],
}

impl fmt::Display for HashedBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "blockdata: {} \nhash : {:?}",
        self.blockdata, self.hash)
    }
}

impl HashedBlock {
    pub fn new(prev_hash : [u8;32], txes : Vec<[u8;32]>) -> Result<Self, QanError> {
        let blockdata = BlockData::new(prev_hash, txes)?;
        let hash = do_hash(&serde_json::to_vec(&blockdata).map_err(|e|QanError::Serde(e))?);
        Ok(HashedBlock{
            blockdata,
            hash 
        })
    }

    /// getter for block timestamp
    pub fn timestamp(&self) -> u64 {
        self.blockdata.timestamp()
    }

}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Block {
    pub hashedblock : HashedBlock,
    pub proposer_pub: [u8;32],
    pub sig         : Vec<u8>,
    pub height      : u64,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {  
        write!(f, "hashedblock : {}\nsig : {}\n",
        self.hashedblock, encode(&self.sig))
    }
}


impl Block{
    #[cfg(not(feature = "quantum"))]
    pub fn new(prev_hash: [u8;32], txes: Vec<[u8;32]>, kp: &Keypair, height : u64) -> Result<Self, QanError> {
        let hashedblock = HashedBlock::new(prev_hash, txes)?;
        let sig = kp.sign(&serde_json::to_vec(&hashedblock).map_err(|e|QanError::Serde(e))?).to_bytes().to_vec();
        let proposer_pub = do_hash(&kp.public.to_bytes().to_vec());
        Ok(Block{
            proposer_pub,
            hashedblock, 
            height,
            sig
        })
    }

    #[cfg(feature = "quantum")]
    pub fn new(prev_hash: [u8;32], txes: Vec<[u8;32]>, sk: &GlpSk, height : u64) -> Result<Self, QanError> {
        let hashedblock = HashedBlock::new(prev_hash, txes)?;
        let sig = sign(&sk, serde_json::to_vec(&hashedblock).map_err(|e|QanError::Serde(e))?).unwrap().to_bytes();
        let proposer_pub = do_hash(&gen_pk(&sk).to_bytes().to_vec());
        Ok(Block{
            proposer_pub,
            hashedblock, 
            height,
            sig
        })
    }

    /// block verification function
    #[cfg(feature = "quantum")]
    pub fn verify(&self, pk : &GlpPk) -> Result<bool, QanError> {
        Ok(verify(&pk, &GlpSig::from_bytes(&self.sig), &serde_json::to_vec(&self.hashedblock).map_err(|e|QanError::Serde(e))?))
    }

    /// block verification function
    #[cfg(not(feature = "quantum"))]
    pub fn verify(&self, pk : &PublicKey) -> Result<bool, QanError>{
        let sig = Signature::from_bytes(&self.sig).unwrap();
        Ok(match pk.verify(&serde_json::to_vec(&self.hashedblock).map_err(|e|QanError::Serde(e))?, &sig){
            Ok(_)=>true,
            Err(_)=>false
        })
    }

    /// Block validation function. Checks if this block 
    ///  was made after the parameter,
    ///  hat a height higher than the parameter,
    ///  is built on the same block determined in the parameter  
    pub fn validate(&self, timestamp: u64, height: u64, prev_hash: [u8;32]) -> (bool, bool, bool){
        ( timestamp < self.timestamp(), height < self.height, prev_hash == self.hashedblock.blockdata.prev_hash )
    }

    /// getter for block hash
    pub fn hash(&self)->[u8;32]{
        self.hashedblock.hash
    }

    /// getter for block merkle root
    pub fn merkle(&self)->Vec<u8>{
        self.hashedblock.blockdata.merkle_root.clone()
    }

    /// getter for previous block hash
    pub fn prev_hash(&self) -> [u8;32]{
        self.hashedblock.blockdata.prev_hash
    }

    /// getter for block timestamp
    pub fn timestamp(&self)->u64{
        self.hashedblock.timestamp()
    }
}

#[test]
fn merk() {
    use static_merkle_tree;
    let v = vec![vec![1,2], vec![1,3]];
    let tree = static_merkle_tree::Tree::from_hashes(v,merge);
    let root : Vec<u8> = tree.get_root_hash().unwrap().to_vec();
    let ret = [85, 83, 19, 65, 189, 78, 73, 18, 202, 219, 205, 133, 143, 168, 181, 5, 137, 77, 197, 123, 49, 124, 243, 20, 206, 207, 161, 3, 90, 131, 240, 91]; 
    assert_eq!(root,ret.to_vec());
}