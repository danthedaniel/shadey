#version 140

attribute vec2 position;
attribute vec2 tex_coords;
varying vec2 pos;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    pos = tex_coords;
}
