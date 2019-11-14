#version 330 core

out vec3 FragPos;
out vec3 Normal;

uniform mat4 projectionMatrix;
uniform mat4 viewMatrix;
uniform mat4 modelMatrix;

in vec4 in_Position;

void main(){

    FragPos = vec3(5.0, 5.0, 1.0);
    Normal = vec3(1.0, 1.0, 1.0);

    gl_Position = projectionMatrix * viewMatrix * modelMatrix * in_Position;
}