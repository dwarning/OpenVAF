use sourcegen::{add_preamble, ensure_file_contents, project_root, reformat};
use xshell::cmd;

#[test]
fn gen_ffi() {
    let vae_dir = project_root().join("crates/verilogae");

    // rustc_bootstrap is used to allow macro expansion on stable. It ain't pretty but its fine here since
    // we don't actually use this to compile code and the generated code is all handchecked and commit
    // the vcs

    let cpp_cfg = project_root().join("crates/verilogae_ffi/cppbindgen.toml");
    let cpp_header = project_root().join("include/verilogae.hpp");
    let cpp_header_content =
        cmd!("cbindgen {vae_dir} -c {cpp_cfg}").env("RUSTC_BOOTSTRAP", "1").read().unwrap();
    ensure_file_contents(&cpp_header, &cpp_header_content);

    let c_cfg = project_root().join("crates/verilogae_ffi/cbindgen.toml");
    let c_header = project_root().join("include/verilogae.h");
    let c_header_content =
        cmd!("cbindgen {vae_dir} -c {c_cfg}").env("RUSTC_BOOTSTRAP", "1").read().unwrap();
    ensure_file_contents(&c_header, &c_header_content);

    let res = cmd!("bindgen {cpp_header} --no-layout-tests --disable-name-namespacing --allowlist-function vae::verilogae_.*
--rustified-enum vae::OptLevel --blacklist-type=vae::NativePath --blacklist-type=vae::FatPtr --blacklist-type=vae::Meta  --allowlist-var=vae::PARAM_FLAGS.* --disable-header-comment").read().unwrap();
    let mut off = 0;
    for line in res.split_terminator('\n') {
        if line.contains("pub type") && (line.contains("__uint") || line.contains("__int")) {
            off += line.len() + 1
        } else {
            break;
        }
    }
    let file_string = format!("{}\n{}", "use super::{NativePath, FatPtr};", &res[off..]);
    let file_string = add_preamble("gen_ffi", reformat(file_string));
    let file = project_root().join("crates/verilogae_ffi/src/ffi.rs");
    ensure_file_contents(&file, &file_string);
}