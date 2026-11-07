uniform sampler2D u_from;
uniform sampler2D u_to;
uniform PAINT_PRECISION float u_alpha;
varying PAINT_PRECISION vec2 v_uv;
void main() {
    // The new end over the old at the fade's opacity, as two layers would blend them.
    gl_FragColor = mix(texture2D(u_from, v_uv), texture2D(u_to, v_uv), u_alpha);
}
