#![allow(static_mut_refs)]
extern crate proc_macro;

extern crate darling;
extern crate syn;
use darling::ast::NestedMeta;
use darling::{Error, FromMeta};
use proc_macro::TokenStream;

static mut OPCODES: Vec<OpcodeArgs> = vec![];

#[derive(Default, FromMeta, Clone)]
#[darling(default)]
struct OpcodeArgs {
    codes: Vec<u8>,
    name: String,
    addr_mode: bool,
}

#[proc_macro_attribute]
pub fn opcode(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(args) => args,
        Err(e) => {
            return TokenStream::from(Error::from(e).write_errors());
        }
    };

    let mut args = match OpcodeArgs::from_list(&args) {
        Ok(args) => args,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let input = item.clone();
    let input = syn::parse_macro_input!(input as syn::ItemFn);
    let func_name = input.sig.ident.to_string();
    args.name = func_name;
    unsafe {
        OPCODES.push(args);
    }
    item
}

#[proc_macro]
pub fn match_all(item: TokenStream) -> TokenStream {
    let mut func_string = String::new();
    func_string.push_str(&format!("match {} {{\n", item.to_owned()));
    unsafe {
        for opcode in OPCODES.clone() {
            // func_string.push_str("self.");
            for code in &opcode.codes {
                func_string.push_str(&format!("0x{:02X}", code));
                func_string.push_str(" | ");
            }
            func_string = func_string.strip_suffix(" | ").unwrap().to_string();
            func_string.push_str(" => { self.");
            func_string.push_str(&opcode.name);
            if opcode.addr_mode {
                func_string.push_str("(&opcode.addr_mode); }\n")
            } else {
                func_string.push_str("(); }\n");
            }
        }
    }
    func_string.push_str(
        format!(
            "_ => panic!(\"Unknown opcode: 0x{{:02X}}\", {})",
            item.to_owned()
        )
        .as_str(),
    );
    func_string.push_str("\n}");
    func_string.parse().unwrap()
    // "0x00 => brk(),".parse().unwrap()
}
