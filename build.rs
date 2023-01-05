fn main() {
    let target = std::env::var("TARGET").unwrap();

    if target.contains("android") {
        println!("cargo:rustc-link-lib=GLESv2");
    }
    #[cfg(feature = "gl_generator")]
    generate_gl();
}

#[cfg(feature = "gl_generator")]
fn generate_gl() {
    use std::{env, fs::File, path::Path};

    use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};

    let dest = env::var("OUT_DIR").unwrap();

    #[cfg(feature = "opengl")]
    {
        let mut file = File::create(&Path::new(&dest).join("gl_bindings.rs")).unwrap();
        Registry::new(Api::Gl, (4, 6), Profile::Core, Fallbacks::All, [])
            .write_bindings(GlobalGenerator, &mut file)
            .unwrap();
    }

    #[cfg(feature = "opengles")]
    {
        let mut file = File::create(&Path::new(&dest).join("gles_bindings.rs")).unwrap();
        Registry::new(Api::Gles2, (2, 0), Profile::Core, Fallbacks::All, [])
            .write_bindings(GlobalGenerator, &mut file)
            .unwrap();
    }
}
