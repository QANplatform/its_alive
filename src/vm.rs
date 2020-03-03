use std::{
	fs::File,
	collections::HashMap,
	io::prelude::*,
	fmt
};
use rmps::{Serializer, Deserializer};
use serde::{Serialize, Deserialize};
use jsonrpc_http_server::jsonrpc_core::Value;

use std::convert::TryInto;
use wasmi::{ImportsBuilder, Module, ModuleInstance, NopExternals, RuntimeValue, *};
use crate::watparser;

#[derive(Serialize, Deserialize, Debug)]
struct Account {
	nonce: u32,
	balance: u64,
	storage_root: String,
	code_hash: String,
	spice_price: u64,
	spice_limit: u64,
	to: Vec<u8>,
	value: u64,
	v: String,
	r: String,
	s: String,
	init: Vec<u8>,
	data: Vec<u8>
}

#[derive(Debug)]
pub struct VM {
	smart_contracts : HashMap<String, Vec<u8>>,
}

#[derive(Debug)]
pub struct CallParams {
	balance: f32,
	address: Vec<u8>,
	caller: Vec<u8>,
	origin: Vec<u8>,
	spice_limit: f32,
	spice_remaining: f32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VMReturn{
	Hash(String),
	Chars(Vec<char>),
	U32(u32),
	U64(u64),
	I64(u64),
	F64(f64),
}

impl VMReturn {
	pub fn ser(&self)->Vec<u8>{
		let mut buf = Vec::new();
        &self.serialize(&mut Serializer::new(&mut buf)).expect("failed to serialize VMReturn");
        buf
	}

	pub fn deser(s : &Vec<u8>)-> Self{
		Deserialize::deserialize(&mut Deserializer::new(&s[..])).expect("failed to deserialize VMReturn")
	}
}

impl fmt::Display for VMReturn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self{
			Self::Hash(d)			=> {write!(f, "{}", d)},
			Self::Chars(d)			=> {write!(f, "{:#?}", d)},
			Self::U32(d)			=> {write!(f, "{}", d)},
			Self::U64(d)			=> {write!(f, "{}", d)},
			Self::I64(d)			=> {write!(f, "{}", d)},
			Self::F64(d)			=> {write!(f, "{}", d)},
		}
    }
}

impl VM{
	pub fn new() -> Self {
		VM{
			smart_contracts : HashMap::new(),
		}
	}

	pub fn add_contract(&mut self, module: Vec<u8>) -> String{
		let instance = Module::from_buffer(&module).unwrap();
		let mod_instance = ModuleInstance::new(&instance, &ImportsBuilder::default())
			.expect("Instantiation failed.")
			.run_start(&mut NopExternals)
			.expect("Start function not found.");

			// hash index
		let index: u32 = u32::from_runtime_value(
			mod_instance.invoke_export("hash_index", &[], &mut NopExternals).expect("addc2").expect("addc3")
		).expect("addc4");

		// hash length
		let length: usize = i32::from_runtime_value(
			mod_instance.invoke_export("hash_len", &[], &mut NopExternals).expect("addc5").expect("addc6")
		).expect("addc7") as usize;
		
		let hash : Vec<char> = mod_instance
			.export_by_name("memory")
			.expect("memory export not f")
			.as_memory().expect("not memory type")
			.get(index, length)
			.expect("oob")
			.iter()
			.map(|el| el.to_owned() as char)
			.collect();
		
		let hastring : String = hash.into_iter().collect();
		self.smart_contracts.insert(hastring.clone(), module);
		println!("{}", hastring);
		hastring
	}

	pub fn build_from_file(&mut self, loadp : String)->String{
		println!("Trying to load smart contract from file: {:?}", loadp);
		let sc = Self::load_file_contract(&loadp);
		let ret = self.add_contract(sc);
		ret
	}

	pub fn load_file_contract(f: &str) -> Vec<u8>{
		let mut file = File::open(f).unwrap();
		let mut buf : Vec<u8>= Vec::new();
		file.read_to_end(&mut buf).unwrap();
		let price = watparser::parse(&buf);
		buf

		//Module::from_buffer(buf).unwrap()
	}

	pub fn handle_rpc_in(inc : Vec<Value>)->Option<(String,String,Vec<RuntimeValue>)>{
		match inc.len(){
			0|1=>return None,
			_ => {
				let (sc, fun) = if inc[0].is_string() && inc[1].is_string(){
					(inc[0].as_str().unwrap().to_string(), inc[1].as_str().unwrap().to_string())
				}else {return None};
				let mut ret = Vec::new();
				if inc.len()>2{
					for v in &inc[2..]{
						match v {
							Value::Number(n)=>{
								if n.is_u64(){
									ret.push(RuntimeValue::I64(n.as_u64().unwrap().try_into().unwrap()));
									continue
								}
								if n.is_i64(){
									ret.push(RuntimeValue::I64(n.as_i64().unwrap()));
									continue
								}
								// if n.is_f64(){
								// 	ret.push(RuntimeValue::F64(n.as_f64().unwrap()));
								// 	continue
								// }
							},
							_=>{},
						}
					}
				}
				Some((sc,fun,ret))
			}
		}
	}

	pub fn call_fun(&self, sc_hash : String, fun_hash: String, params : Vec<RuntimeValue>) -> VMReturn{

		let account_in =  Account {
			nonce: 0,
			balance: 9999,
			storage_root: "".to_string(),
			code_hash: "".to_string(),
			spice_price: 1,
			spice_limit: 10,
			to: Vec::new(),
			value: 1,
			v: "".to_string(),
			r: "".to_string(),
			s: "".to_string(),
			init: Vec::new(),
			data: Vec::new()
		};
	
		let contract = Module::from_buffer(self.smart_contracts.get(&sc_hash).expect("callfun1")).expect("callfun2");
		let loaded_module = ModuleInstance::new(&contract, &ImportsBuilder::default())
							.expect("Instantiation failed")
							.run_start(&mut NopExternals)
							.expect("No main function");
		
		let invoked = loaded_module
			.invoke_export(&fun_hash, &params, &mut NopExternals).expect("callfun3").expect("callfun4");

		let is_string: bool = (match loaded_module
			.invoke_export( &(format!("{}_index", &fun_hash)), &params, &mut NopExternals )
			{
				Ok(x) => true,
				Err(x) => false
			}) && (match loaded_module
				.invoke_export( &(format!("{}_len", &fun_hash)), &params, &mut NopExternals )
				{
					Ok(x) => true,
					Err(x) => false
				}
			);

		if is_string {
			let index: u32 = u32::from_runtime_value( loaded_module
				.invoke_export( &(format!("{}_index", &fun_hash)), &params, &mut NopExternals ).expect("no index export").expect("no index??")
				).expect("not u32");
			let len: usize = u32::from_runtime_value( loaded_module
				.invoke_export( &(format!("{}_len", &fun_hash)), &params, &mut NopExternals ).expect("no lenght export").expect("no length??")
				).expect("not u32 2") as usize;

			let str: Vec<char> = loaded_module
				.export_by_name("memory").expect("memory export failed").as_memory().expect("OOB error").get(index, len).expect("OOB error 2")
				.iter().map(|x| *x as char).collect();

			return VMReturn::Chars(str);
		}else{
			let ret : VMReturn = match invoked{
				RuntimeValue::I32(i)=>VMReturn::U32(i.try_into().unwrap()),
				RuntimeValue::I64(i)=>VMReturn::U64(i.try_into().unwrap()),
				RuntimeValue::F32(f)=>VMReturn::F64(f.to_float().into()),
				RuntimeValue::F64(f)=>VMReturn::F64(f.to_float()),
			};
			println!("SmartContract: \"{}\" has run its course, with function: \"{}\" and has given the result: {:?}",sc_hash, fun_hash, ret);
			return ret;
		}
		// let length: usize = i32::from_runtime_value(
		// 	mod_instance.invoke_export("hash_len", &[], &mut NopExternals).unwrap().unwrap()
		// ).unwrap() as usize;
	}
}