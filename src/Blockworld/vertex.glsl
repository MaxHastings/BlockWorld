#version 120

attribute vec3 in_Position;
attribute vec3 in_Normal;
attribute vec2 in_TexCoord;

uniform mat4 projectionMatrix;
uniform mat4 viewMatrix;
uniform mat4 modelMatrix;

varying vec3 FragPos;
varying vec3 Normal;
varying vec2 TexCoord;
varying float viewDepth;

void main() {
    vec4 worldPosition = modelMatrix * vec4(in_Position, 1.0);
    vec4 eyePosition = viewMatrix * worldPosition;

    FragPos = worldPosition.xyz;
    Normal = normalize(mat3(modelMatrix) * in_Normal);
    TexCoord = in_TexCoord;
    viewDepth = abs(eyePosition.z);
    gl_Position = projectionMatrix * eyePosition;
}
