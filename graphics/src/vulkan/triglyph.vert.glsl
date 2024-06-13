#version 460

layout(location = 0) in vec4 vertexData; // (vertCoordX, vertCoordY, textureCoordX, textureCoordY)
layout(location = 0) out vec2 textureCoord; // Which will be interpolated by graphics pipeline

layout(set = 0, binding = 3) uniform UniformData {
	vec2 mult;
} uni;

const vec2 offset = vec2(-1.0, -1.0);

void main() {
    textureCoord = vertexData.zw;
    gl_Position = vec4((vertexData.xy * uni.mult) + offset, 0.0, 1.0);
}
