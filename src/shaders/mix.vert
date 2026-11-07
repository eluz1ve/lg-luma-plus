// A wallpaper fade's two pictures, laid over the whole drawable in one pass.
attribute vec2 a_position;
varying PAINT_PRECISION vec2 v_uv;
void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
    // The pictures were drawn in this drawable's orientation: clip space maps straight on.
    v_uv = a_position * 0.5 + 0.5;
}
