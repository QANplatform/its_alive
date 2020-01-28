#[derive(Debug, PartialEq, Deserialize, Serialize, Eq, Hash, Clone)]
pub struct ConsensusSettings {
    min_tx      :   usize,
    min_size    :   usize,
    min_time    :   u64,
}

impl ConsensusSettings {
    pub fn new( min_tx : usize , min_size : usize , min_time : u64 ) -> Self {
        ConsensusSettings{ min_tx , min_size , min_time }
    }

    pub fn default() -> Self{
        ConsensusSettings{
            min_tx      :   10,
            min_size    :   1000,
            min_time    :   10,
        }
    }

    pub fn check_limiters(&self, tx_count : usize, pool_size : usize , prev_time : u64 )-> bool {
        if  (self.min_tx<=tx_count) && 
            (self.min_size<pool_size) && 
            (self.min_time<crate::util::timestamp()-prev_time) 
        { 
            println!("{:?}", self);
            return true 
        }
        false
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    pub fn deserialize( s : &str ) -> Self {
        serde_json::from_str(s).unwrap()
    }

    pub fn deserialize_slice( s :&[u8] ) -> Self {
        serde_json::from_slice(s).unwrap()
    }
}