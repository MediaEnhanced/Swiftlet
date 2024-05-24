#version 460

layout(location = 0) in vec4 vertexData; // (vertCoordX, vertCoordY, glyphOutlineCoordX, glyphOutlineCoordY)
layout(location = 0) out vec2 glyphOutlineCoord; // Which will be interpolated by graphics pipeline

void main() {
    glyphOutlineCoord = vertexData.zw;
    gl_Position = vec4(vertexData.xy, 0.0, 1.0);
}

