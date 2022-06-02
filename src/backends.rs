#[cfg(all(feature = "webgl", target_arch = "wasm32"))]
mod webgl;
#[cfg(all(feature = "webgl", target_arch = "wasm32"))]
pub use webgl::WebGLSpriteRender;

#[cfg(all(not(target_arch = "wasm32"), feature = "opengl"))]
mod opengl;
#[cfg(all(not(target_arch = "wasm32"), feature = "opengl"))]
pub use opengl::GLSpriteRender;

#[cfg(all(not(target_arch = "wasm32"), feature = "opengles"))]
mod opengles;
#[cfg(all(not(target_arch = "wasm32"), feature = "opengles"))]
pub use opengles::GlesSpriteRender;
