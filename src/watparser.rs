use wasmprinter::*;

pub fn parse(inp: &Vec<u8>) -> i32 {
    let wat = wasmprinter::print_bytes(inp);
    //h
    42
}