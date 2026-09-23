#version 120

uniform sampler2D textureAtlas;
uniform vec3 lightPos;
uniform vec3 lightColor;
uniform vec3 objectColor;
uniform vec3 fogColor;
uniform float fogStart;
uniform float fogEnd;

varying vec3 FragPos;
varying vec3 Normal;
varying vec2 TexCoord;
varying float viewDepth;

void main() {
    vec3 norm = normalize(Normal);
    vec3 lightDir = normalize(lightPos - FragPos);
    float diffuseStrength = max(dot(norm, lightDir), 0.0);

    vec3 ambient = 0.35 * lightColor;
    vec3 diffuse = 0.65 * diffuseStrength * lightColor;
    vec4 texel = texture2D(textureAtlas, TexCoord);
    vec3 litColor = texel.rgb * objectColor * (ambient + diffuse);

    float visibility = clamp((fogEnd - viewDepth) / (fogEnd - fogStart), 0.0, 1.0);
    gl_FragColor = vec4(mix(fogColor, litColor, visibility), texel.a);
}
