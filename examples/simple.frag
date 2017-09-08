#version 140

in vec2 pos;
out vec4 color;

uniform sampler2D tex;

void main() {
    color = texture(tex, pos);
}
