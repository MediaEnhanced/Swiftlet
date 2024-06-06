#version 460

layout(location = 0) in vec4 vertexData; // (vertCoordX, vertCoordY, textureCoordX, textureCoordY)
layout(location = 0) out vec2 textureCoord; // Which will be interpolated by graphics pipeline

void main() {
    textureCoord = vertexData.zw;
    gl_Position = vec4(vertexData.xy, 0.0, 1.0);
}
