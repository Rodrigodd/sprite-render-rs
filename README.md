## Todo:

- Remove winit (and other dependencies) with empty features
- Add a set_top_left_position for the camera
- make load texture be used with differents formats
- make possible to recreate a texture with different size (and format?)
- make a proper nonzero type for the textures...
- split the sprites in diferrents draw calls when the number of texture is greater than MAX_TEXTURE_IMAGE_UNITS
- 'clamp to edge' or 'repeat'?
- fix ```main.rs``` example: 1 line scroll = 100 pixel scroll

### webgl

- handle the limitation of ~~16384~~ 5400 sprites