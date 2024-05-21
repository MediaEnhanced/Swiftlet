#version 460
#extension GL_EXT_debug_printf : enable

layout (location = 0) in vec2 texCoord;
layout (location = 0) out vec4 fragColor;

struct ColorFontS{
	vec4 linearRGBA;
    uvec4 fontIndex;
};

layout(std430, set = 0, binding = 0) readonly buffer Bound{
	ColorFontS data[];
} primitiveInfo;

layout(std430, set = 0, binding = 1) readonly buffer Bound1{
	uvec4 info;
    uint segment_offsets[];
} fontInfo;

struct GlyphSegment{
	vec4 yValues;
    vec4 xValues;
};

layout(std430, set = 0, binding = 2) readonly buffer Bound3{
	GlyphSegment data[];
} glyphSegments;

const float kQuadraticEpsilon = 0.0001;

void main()
{   
    vec4 color = primitiveInfo.data[gl_PrimitiveID].linearRGBA;
    uint glyph = primitiveInfo.data[gl_PrimitiveID].fontIndex.r;
    //
    // float myfloat = 3.1415;
    // debugPrintfEXT("My float is %f\n", myfloat);
    if (glyph == 0) {
        fragColor = color;
    } else {
        //debugPrintfEXT("UV: (%f, %f)\n", texCoord.r, texCoord.g);
        // bool debug_print = (texCoord.r > 550.0) && (texCoord.r < 750.0) && (texCoord.g > 550.0) && (texCoord.g < 670.0);
        
        vec2 dF = vec2(dFdx(texCoord.r), -dFdy(texCoord.g));
        //debugPrintfEXT("DF: %f, %f\n", dF.r, dF.g);
        //debugPrintfEXT("Font Info: %u, %u\n", fontInfo.info.r, fontInfo.info.g);

        // if (debug_print) {
        //     debugPrintfEXT("DF: %f, %f\n", dF.r, dF.g);
        //     //debugPrintfEXT("DF: %f, %f, %f, %f\n", dFdxCoarse(texCoord.r), dFdxCoarse(texCoord.g), dFdyCoarse(texCoord.r), dFdyCoarse(texCoord.g));
        // }

        uint segment_data_offset = fontInfo.segment_offsets[glyph];
        uint segment_data_length = fontInfo.segment_offsets[glyph + 1] - segment_data_offset;
        //debugPrintfEXT("Offset|NumSegments: %u: %u | %u\n", glyph, segment_data_offset, segment_data_length);

        uint seg = segment_data_offset;
        uint cnt = segment_data_length;

        float xCheck = texCoord.r - (dF.r * 0.5);
        float smpY = texCoord.g;
        float scaler = 1.0 / dF.r;
        float coverage = 0.0;
        while (cnt > 0) {
            vec4 segX = glyphSegments.data[seg].xValues;
            if (segX.r <= xCheck) {
                break;
            }

            vec4 segY = glyphSegments.data[seg].yValues;

            // Check if is quad
            if (segY.r != 0) {
                if (segY.g > smpY) {
                    if (segY.b <= smpY) {
                        float ay = segY.g - (2.0 * segY.a) + segY.b;
                        float by = segY.g - segY.a;
                        float cy = segY.g - smpY;
                        float d = sqrt(max((by * by) - (ay * cy), 0.0));
                        float t1 = (by - d) / ay;
                        // if (abs(ay) < kQuadraticEpsilon) {
                        //     t1 = segY.g * 0.5 / by;
                        // }

                        float ax = segX.g - (2.0 * segX.a) + segX.b;
                        float bx = segX.g - segX.a;
                        float x1 = (ax * t1 - bx * 2.0) * t1 + segX.g;

                        float add_coverage = (x1 - xCheck) * scaler;
                        coverage += clamp(add_coverage, 0.0, 1.0);
                        // if (debug_print) {
                        //     debugPrintfEXT("AC0 %u: %f (%f, %f)\n", cnt, add_coverage, texCoord.r, texCoord.g);
                        // }
                    } else if (segY.a <= smpY) {
                        float ay = segY.g - (2.0 * segY.a) + segY.b;
                        float by = segY.g - segY.a;
                        float cy = segY.g - smpY;
                        float d = sqrt(max((by * by) - (ay * cy), 0.0));
                        float t1 = (by - d) / ay;
                        float t2 = (by + d) / ay;
                        // if (abs(ay) < kQuadraticEpsilon) {
                        //     t1 = t2 = segY.g * 0.5 / by;
                        // }

                        float ax = segX.g - (2.0 * segX.a) + segX.b;
                        float bx = segX.g - segX.a;
                        float x1 = (ax * t1 - bx * 2.0) * t1 + segX.g;
                        float x2 = (ax * t2 - bx * 2.0) * t2 + segX.g;

                        float add_coverage = (x1 - xCheck) * scaler;
                        coverage += clamp(add_coverage, 0.0, 1.0);
                        float sub_coverage = (x2 - xCheck) * scaler;
                        coverage -= clamp(sub_coverage, 0.0, 1.0);
                        // if (debug_print) {
                        //     debugPrintfEXT("AC1 %u: %f\n", cnt, add_coverage);
                        //     debugPrintfEXT("SC1 %u: %f\n", cnt, sub_coverage);
                        // }
                    }
                } else if (segY.b > smpY) {
                    float ay = segY.g - (2.0 * segY.a) + segY.b;
                    float by = segY.g - segY.a;
                    float cy = segY.g - smpY;
                    float d = sqrt(max((by * by) - (ay * cy), 0.0));
                    float t2 = (by + d) / ay;
                    // if (abs(ay) < kQuadraticEpsilon) {
                    //     t2 = segY.g * 0.5 / by;
                    // }

                    float ax = segX.g - (2.0 * segX.a) + segX.b;
                    float bx = segX.g - segX.a;
                    float x2 = (ax * t2 - bx * 2.0) * t2 + segX.g;

                    float sub_coverage = (x2 - xCheck) * scaler;
                    coverage -= clamp(sub_coverage, 0.0, 1.0);
                    // if (debug_print) {
                    //     debugPrintfEXT("SC2 %u: %f (%f, %f)\n", cnt, sub_coverage, texCoord.r, texCoord.g);
                    // }
                } else if (segY.a > smpY) {
                    float ay = segY.g - (2.0 * segY.a) + segY.b;
                    float by = segY.g - segY.a;
                    float cy = segY.g - smpY;
                    float d = sqrt(max((by * by) - (ay * cy), 0.0));
                    float t1 = (by - d) / ay;
                    float t2 = (by + d) / ay;
                    // if (abs(ay) < kQuadraticEpsilon) {
                    //     t1 = t2 = segY.g * 0.5 / by;
                    // }

                    float ax = segX.g - (2.0 * segX.a) + segX.b;
                    float bx = segX.g - segX.a;
                    float x1 = (ax * t1 - bx * 2.0) * t1 + segX.g;
                    float x2 = (ax * t2 - bx * 2.0) * t2 + segX.g;

                    float add_coverage = (x1 - xCheck) * scaler;
                    coverage += clamp(add_coverage, 0.0, 1.0);
                    float sub_coverage = (x2 - xCheck) * scaler;
                    coverage -= clamp(sub_coverage, 0.0, 1.0);
                    // if (debug_print) {
                    //     debugPrintfEXT("AC3 %u: %f\n", cnt, add_coverage);
                    //     debugPrintfEXT("SC3 %u: %f\n", cnt, sub_coverage);
                    // }
                }
            } else {
                if (segY.g > smpY) {
                    if (segY.b <= smpY) {
                        float x = ((smpY - segY.g) * (segX.b - segX.g) / (segY.b - segY.g)) + segX.g;
                        float add_coverage = (x - xCheck) * scaler;
                        coverage += clamp(add_coverage, 0.0, 1.0);
                        // if (debug_print) {
                        //     debugPrintfEXT("AC4 %u: %f\n", cnt, add_coverage);
                        // }
                    }
                } else if (segY.b > smpY) {
                    float x = ((smpY - segY.g) * (segX.b - segX.g) / (segY.b - segY.g)) + segX.g;
                    float sub_coverage = (x - xCheck) * scaler;
                    coverage -= clamp(sub_coverage, 0.0, 1.0);
                    // if (debug_print) {
                    //     debugPrintfEXT("SC5 %u: %f\n", cnt, sub_coverage);
                    // }
                }
            }

            seg += 1;
            cnt -= 1;
        }

        // if (debug_print) {
        //     debugPrintfEXT("TC %f\n", coverage);
        // }
        float alpha = clamp(abs(coverage), 0.0, 1.0);
        //Maybe discard in future if alpha is fully transparent!
        //Also check alpha * alpha theory
        fragColor = color * alpha;
    }
}

