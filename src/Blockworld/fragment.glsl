#version 330 core
varying vec4 vertColor;

uniform vec3 lightPos = vec3(0.0, 50.0, 0.0);
uniform vec3 lightColor = vec3(1.0, 1.0, 1.0);

in vec3 Normal;
in vec3 FragPos;

void main(){

    float ambientStrength = 0.1;
    vec3 ambient = ambientStrength * lightColor;

    vec3 result = ambient * objectColor;

    // diffuse
    vec3 norm = normalize(Normal);
    vec3 lightDir = normalize(lightPos - FragPos);
    float diff = max(dot(norm, lightDir), 0.0);
    vec3 diffuse = diff * lightColor;

    gl_FragColor = vec4(result, 1.0);
}