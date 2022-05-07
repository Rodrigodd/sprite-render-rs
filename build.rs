fn main() {
    #[cfg(feature = "gl_generator")]
    generate_gl();
}

#[cfg(feature = "gl_generator")]
fn generate_gl() {
    use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};
    use std::env;
    use std::fs::File;
    use std::path::Path;

    let dest = env::var("OUT_DIR").unwrap();
    let mut file = File::create(&Path::new(&dest).join("gl_bindings.rs")).unwrap();

    Registry::new(Api::Gl, (4, 6), Profile::Core, Fallbacks::All, [])
        .write_bindings(GlobalGenerator, &mut file)
        .unwrap();
}
