use std::io::Write;

#[cfg(not(feature = "quantum"))]
pub fn generate_nemezis_block(kp: &ed25519_dalek::Keypair){
    let ConsensusSettings = crate::conset::ConsensusSettings::default();
    let nemezis_body = crate::transaction::TxBody::new([0;32], ConsensusSettings.serialize().as_bytes().to_vec()); 
    let nemesis_tx = crate::transaction::Transaction::new(nemezis_body, &kp);
    let mut nemezis_vec = Vec::new();
    nemezis_vec.push(nemesis_tx.hash());
    let block = crate::block::Block::new(hex::encode([0;32]), nemezis_vec, &kp);
    let mut pemf = std::fs::File::create(std::path::Path::new("NEMEZIS")).unwrap();
    pemf.write_fmt(format_args!("{}", serde_json::to_string(&block).unwrap()));
}

#[cfg(feature = "quantum")]
pub fn generate_nemezis_block(kp: &glp::glp::GlpSk){
    let ConsensusSettings = crate::conset::ConsensusSettings::default();
    let nemezis_body = crate::transaction::TxBody::new([0;32], ConsensusSettings.serialize().as_bytes().to_vec()); 
    let nemesis_tx = crate::transaction::Transaction::new(nemezis_body, &kp);
    let mut nemezis_vec = Vec::new();
    nemezis_vec.push(nemesis_tx.hash());
    let block = crate::block::Block::new(hex::encode([0;32]), nemezis_vec, &kp);
    let mut pemf = std::fs::File::create(std::path::Path::new("qNEMEZIS")).unwrap();
    pemf.write_fmt(format_args!("{}", serde_json::to_string(&block).unwrap()));
}