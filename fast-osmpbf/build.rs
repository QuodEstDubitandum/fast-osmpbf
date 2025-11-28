use pb_rs::{ConfigBuilder, types::FileDescriptor};
use std::{env, fs};

fn main() {
    let out_dir_root = env::var("OUT_DIR").unwrap();
    let out_dir = format!("{}/proto", out_dir_root);
    let in_dir = format!("{}/proto", env::var("CARGO_MANIFEST_DIR").unwrap());

    // Tell Cargo when to rebuild
    println!("cargo:rerun-if-changed={}/osmdata.proto", in_dir);
    println!("cargo:rerun-if-changed={}/osmformat.proto", in_dir);

    if fs::metadata(&out_dir).is_ok() {
        fs::remove_dir_all(&out_dir).unwrap();
    }
    fs::create_dir_all(&out_dir).unwrap();

    let mut protos = vec![];
    protos.push(format!("{}/{}", in_dir, "osmdata.proto"));
    protos.push(format!("{}/{}", in_dir, "osmformat.proto"));

    let config = ConfigBuilder::new(&protos, None, Some(&&out_dir), &[in_dir])
        .expect("could not generate pb-rs config");

    let descriptor = config.build();
    FileDescriptor::run(&descriptor).expect("could not generate proto files");
}
