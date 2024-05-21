#version 460
//#extension GL_EXT_debug_printf : enable

layout(location = 0) in vec4 vertexData;
layout(location = 0) out vec2 texCoord;

void main() {
    //debugPrintfEXT("Processing: (%f, %f)\n", vertexData.r, vertexData.g);
    texCoord = vertexData.ba; // Interpolated
    gl_Position = vec4(vertexData.rg, 0.0, 1.0);
}

