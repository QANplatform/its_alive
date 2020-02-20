use serde::{Serialize, Deserialize};
use rmps::{Serializer, Deserializer};
use crate::util::blake2b;
#[cfg(not(feature = "quantum"))]
use ed25519_dalek::{Keypair, PublicKey, Signature};
#[cfg(feature = "quantum")]
use glp::glp::{GlpSig, GlpSk, GlpPk, sign, verify, gen_pk};
use hex::encode;
use std::fmt;

pub fn merge(l:&[u8;32],r:&[u8;32])->[u8;32]{
    let mut buf = Vec::new();
    buf.extend_from_slice(l);
    buf.extend_from_slice(r);
    blake2b(&buf) 
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Eq, Hash, Clone)]
pub struct BlockData {
    pub timestamp   : u64,
    pub merkle_root : Vec<u8>,
    pub prev_hash   : String,
    pub txes        : Vec<[u8;32]>,
}

impl fmt::Display for BlockData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tx_bases = Vec::new();
        for i in &self.txes{
            tx_bases.push(encode(&i));
        } 
        write!(f, "timestamp : {}\nmerkle_root : {}\nprev_hash : {}\ntxes : {:?}",
        self.timestamp, encode(&self.merkle_root), self.prev_hash, tx_bases)
    }
}

impl BlockData {
    pub fn new(prev_hash : String, txes : Vec<[u8;32]>) -> Self {
        let tree = static_merkle_tree::Tree::from_hashes(txes.clone(),merge);
        let merkle_root : Vec<u8> = tree.get_root_hash().unwrap().to_vec();
        BlockData{
            prev_hash,
            timestamp: crate::util::timestamp(),
            merkle_root,
            txes,
        }
    }

    pub fn timestamp(&self) -> u64{
        self.timestamp
    }

    pub fn block_to_blob(&self)->Vec<u8>{
        let mut block_buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut block_buf)).expect("block_to_blob");
        block_buf
    }

}

#[derive(Debug, PartialEq, Deserialize, Serialize, Eq, Hash, Clone)]
pub struct HashedBlock {
    pub blockdata   : BlockData,
    pub hash        : String,
}

impl fmt::Display for HashedBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "blockdata: {} \nhash : {}",
        self.blockdata, self.hash)
    }
}

impl HashedBlock {
    pub fn new(prev_hash : String, txes : Vec<[u8;32]>) -> Self {
        let blockdata = BlockData::new(prev_hash, txes);
        let hash = encode(blake2b(&blockdata.block_to_blob()));
        HashedBlock{
            blockdata,
            hash 
        }
    }

    pub fn timestamp(&self) -> u64 {
        self.blockdata.timestamp()
    }

    pub fn block_to_blob(&self)->Vec<u8>{
        let mut block_buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut block_buf)).expect("block_to_blob");
        block_buf
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Eq, Hash, Clone)]
pub struct Block {
    pub hashedblock : HashedBlock,
    pub proposer_pub: Vec<u8>,
    pub sig         : Vec<u8>,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {  
        write!(f, "hashedblock : {}\nsig : {}\n",
        self.hashedblock, encode(&self.sig))
    }
}


impl Block{
    #[cfg(not(feature = "quantum"))]
    pub fn new(prev_hash: String, txes: Vec<[u8;32]>, kp: &Keypair) -> Block {
        let hashedblock = HashedBlock::new(prev_hash, txes);
        let sig = kp.sign(&hashedblock.block_to_blob()).to_bytes().to_vec();
        let proposer_pub = kp.public.to_bytes().to_vec();
        Block{
            proposer_pub,
            hashedblock, 
            sig
        }
    }

    #[cfg(feature = "quantum")]
    pub fn new(prev_hash: String, txes: Vec<[u8;32]>, sk: &GlpSk) -> Block {
        let hashedblock = HashedBlock::new(prev_hash, txes);
        let sig = sign(&sk, hashedblock.block_to_blob()).unwrap().to_bytes();
        let proposer_pub = gen_pk(&sk).to_bytes().to_vec();
        Block{
            proposer_pub,
            hashedblock, 
            sig
        }
    }

    #[cfg(feature = "quantum")]
    pub fn validate(&self) -> bool {
        let pk = GlpPk::from_bytes(&self.proposer_pub);
        verify(&pk, GlpSig::from_bytes(&self.sig), self.hashedblock.block_to_blob()) 
    }

    #[cfg(not(feature = "quantum"))]
    pub fn validate(&self) -> bool{
        let pk = PublicKey::from_bytes(&self.proposer_pub).unwrap();
        let sig = Signature::from_bytes(&self.sig).unwrap();
        match pk.verify(&self.hashedblock.block_to_blob(), &sig){
            Ok(_)=>true,
            Err(_)=>false
        }
    }
    pub fn hash(&self)->String{
        self.hashedblock.hash.clone()
    }

    pub fn timestamp(&self)->u64{
        self.hashedblock.timestamp()
    }

    pub fn block_to_blob(&self)->Vec<u8>{
        let mut block_buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut block_buf)).expect("block_to_blob");
        block_buf
    }

    pub fn block_from_vec(v : &Vec<u8>)-> Block{
        Deserialize::deserialize(&mut Deserializer::new(&v[..])).expect("block_from_vec")
    } 
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Eq, Hash, Clone)]
pub struct SyncBlock {
    pub block : Block,
    pub height: String,
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