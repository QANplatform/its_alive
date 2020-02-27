use std::io::prelude::*;
use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};
use blake2::{Blake2b, /*Blake2s,*/ Digest as BDigest};

pub fn timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("timestamp").as_millis() as u64
}

pub fn urandom(n: usize) -> Vec<u8> {
    let mut fd = File::open("/dev/urandom").expect("failed to open urandom");
    let mut ret = vec![0; n];
    fd.read(&mut ret).expect("failed to read urandom");
    println!("urandom in new line:\n {:?}", ret);
    ret.to_vec()
}

pub fn hex_arr_to_dec_arr(){
    //79,33,fd,e3,32,43,76,e6,92,52,ce,06,cc,86,5b,23,8e,c5,29,36,0e,7e,59,6a,ee,e0,3f,97,6c,5c,8d,08
    fn match_i(i: &str)->u8{match i{"0"=>0,"1"=>16,"2"=>32,"3"=>48,"4"=>64,"5"=>80,"6"=>96,"7"=>112,"8"=>128,"9"=>144,"a"=>160,"b"=>176,"c"=>192,"d"=>208,"e"=>224,"f"=>240,_=>0,}}
    fn match_j(j: &str)->u8{match j{"0"=>0,"1"=>1,"2"=>2,"3"=>3,"4"=>4,"5"=>5,"6"=>6,"7"=>7,"8"=>8,"9"=>9,"a"=>10,"b"=>11,"c"=>12,"d"=>13,"e"=>14,_=>0,}}
    let hash = [("7","9"),("3","3"),("f","d"),("e","3"),("3","2"),("4","3"),("7","6"),("e","6"),("9","2"),("5","2"),("c","e"),("0","6"),("c","c"),("8","6"),("5","b"),("2","3"),
                ("8","e"),("c","5"),("2","9"),("3","6"),("0","e"),("7","e"),("5","9"),("6","a"),("e","e"),("e","0"),("3","f"),("9","7"),("6","c"),("5","c"),("8","d"),("0","8")];
    let mut ret : Vec<u8> = Vec::new();
    for (i,j) in hash.iter(){ret.push(match_i(i)+match_j(j));}
    println!("{:?}", ret);
}

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

pub fn vec_to_arr(v : &Vec<u8>)->[u8;32]{
    let mut a : [u8;32] =[0;32];
    a.clone_from_slice(&v[0..32]);
    a
}