use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
    // --nmagic and -Tlink.x are already set via [target.thumbv6m-none-eabi]
    // rustflags in .cargo/config.toml; repeating them here made the linker
    // process `INCLUDE memory.x` twice, which fails with "region 'BOOT2'
    // already defined".
    //
    // -Tdefmt.x used to be emitted here. It is gone along with defmt itself:
    // the RP2040-Zero exposes no SWD pads, so there is no probe to carry RTT.
}
