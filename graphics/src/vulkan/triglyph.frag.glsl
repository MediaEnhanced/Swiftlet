#version 460
//#extension GL_EXT_debug_printf : enable

layout (location = 0) in vec2 textureCoord; // Which is an interpolated value
layout (location = 0) out vec4 preMultipliedAlphaColorOutput;

struct PrimitiveInfo {
	vec4 linearRGBA; // Fragment Pre-Multiplied Alpha Color in Linear RGB Space
    uvec4 glyphOutlineInfo; // Fragment Glyph Outline Info: (outlineIndex, po2RaysPerFragment, reserved0, reserved1)
};

layout(std430, set = 0, binding = 0) readonly buffer Buffer0 {
	PrimitiveInfo info[];
} primitive;

layout(std430, set = 0, binding = 1) readonly buffer Buffer1 {
    uint numOutlines;
    uint po2RaysPerOutline; // As a Power of 2 meaning: 0 => 1 Ray | 1 => 2 Rays | 2 => 4 Rays | 3 => 8 Rays | etc...
    uint reserved0;
    uint reserved1;
    uint segmentOffsets[];
} glyphOutlines;

struct GlyphOutlineSegment{
	vec4 yValues;
    vec4 xValues;
};

layout(std430, set = 0, binding = 2) readonly buffer Buffer2 {
	GlyphOutlineSegment data[];
} glyphOutlineSegments;

// Degrees: 0, 90, -45, 45, -67.5, -22.5, 22.5, 67.5
const float cosPreCalc[8] = float[](1.0, 0.0, 0.707106781187, 0.707106781187, 0.382683432365, 0.923879532511, 0.923879532511, 0.382683432365);
const float sinPreCalc[8] = float[](0.0, 1.0, -0.707106781187, 0.707106781187, -0.923879532511, -0.382683432365, 0.382683432365, 0.923879532511);
const float xTestOffset[8] = float[](0.5, 0.5, 0.707106781187, 0.707106781187, 0.541196100146, 0.541196100146, 0.541196100146, 0.541196100146);
const float kQuadraticEpsilon = 0.0001;

void main()
{   
    PrimitiveInfo primitiveInfo = primitive.info[gl_PrimitiveID];
    vec4 color = primitiveInfo.linearRGBA;
    uint outlineIndex = primitiveInfo.glyphOutlineInfo.x;

    if (outlineIndex >= glyphOutlines.numOutlines) {
        preMultipliedAlphaColorOutput = color;
    } else {
        uint segmentStartIndex = glyphOutlines.segmentOffsets[outlineIndex];
        uint numSegments = glyphOutlines.segmentOffsets[outlineIndex + 1] - segmentStartIndex;
        uint numSegmentsPerRay = numSegments >> glyphOutlines.po2RaysPerOutline;

        vec2 dF = vec2(dFdx(textureCoord.x), -dFdy(textureCoord.y));

        float coverage = 0.0;
        float scaler = 1.0 / dF.x;
        vec2 samplePart1 = vec2(-textureCoord.y, textureCoord.x);

        uint po2RaysPerFragment = min(primitiveInfo.glyphOutlineInfo.y, glyphOutlines.po2RaysPerOutline);
        po2RaysPerFragment = min(po2RaysPerFragment, 3);
        uint rayIndex = 0;
        uint numRays = 1 << po2RaysPerFragment;
        float avg_div = float(numRays);
        //float avg_mult = 1.0 / float(numRays);

        // bool debug_print = numRays > 2;
        // if (debug_print) {
        //     avg_mult = 1.0;
        //     //debugPrintfEXT("NR: (%u, %f)\n", numRays, textureCoord.g);
        // }
        
        while (numRays > 0) {
            // if (debug_print && rayIndex != 3) {
            //     rayIndex += 1;
            //     numRays -= 1;
            //     continue;
            // }

            vec2 part0 = textureCoord * cosPreCalc[rayIndex];
            vec2 part1 = samplePart1 * sinPreCalc[rayIndex];
            vec2 s = part0 + part1;
            float xCheck = s.x - (dF.x * xTestOffset[rayIndex]);
            float smpY = s.y;

            uint segmentIndex = (numSegmentsPerRay * rayIndex) + segmentStartIndex;
            uint numSegs = numSegmentsPerRay;
            while (numSegs > 0) {
                GlyphOutlineSegment seg = glyphOutlineSegments.data[segmentIndex];
                vec4 segX = seg.xValues;
                if (segX.r <= xCheck) {
                    break;
                }

                vec4 segY = seg.yValues;
                // Check if is quad
                if (segY.r != 0) {
                    if (segY.g > smpY) {
                        if (segY.b <= smpY) {
                            float ay = segY.g - (2.0 * segY.a) + segY.b;
                            float by = segY.g - segY.a;
                            float cy = segY.g - smpY;
                            float d = sqrt(max((by * by) - (ay * cy), 0.0));
                            float t1 = (by - d) / ay;
                            if (abs(ay) < kQuadraticEpsilon) {
                                t1 = cy * 0.5 / by;
                            }

                            float ax = segX.g - (2.0 * segX.a) + segX.b;
                            float bx = segX.g - segX.a;
                            float x1 = (ax * t1 - bx * 2.0) * t1 + segX.g;

                            float add_coverage = (x1 - xCheck) * scaler;
                            coverage += clamp(add_coverage, 0.0, 1.0);
                            // if (debug_print) {
                            //     debugPrintfEXT("AC0 %u: %f (%f, %f)\n", cnt, add_coverage, textureCoord.r, textureCoord.g);
                            // }
                        } else if (segY.a <= smpY) {
                            float ay = segY.g - (2.0 * segY.a) + segY.b;
                            float by = segY.g - segY.a;
                            float cy = segY.g - smpY;
                            float d = sqrt(max((by * by) - (ay * cy), 0.0));
                            float t1 = (by - d) / ay;
                            float t2 = (by + d) / ay;
                            if (abs(ay) < kQuadraticEpsilon) {
                                t1 = t2 = cy * 0.5 / by;
                            }

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
                        if (abs(ay) < kQuadraticEpsilon) {
                            t2 = cy * 0.5 / by;
                        }

                        float ax = segX.g - (2.0 * segX.a) + segX.b;
                        float bx = segX.g - segX.a;
                        float x2 = (ax * t2 - bx * 2.0) * t2 + segX.g;

                        float sub_coverage = (x2 - xCheck) * scaler;
                        coverage -= clamp(sub_coverage, 0.0, 1.0);
                        // if (debug_print) {
                        //     debugPrintfEXT("SC2 %u: %f (%f, %f)\n", cnt, sub_coverage, textureCoord.r, textureCoord.g);
                        // }
                    } else if (segY.a > smpY) {
                        float ay = segY.g - (2.0 * segY.a) + segY.b;
                        float by = segY.g - segY.a;
                        float cy = segY.g - smpY;
                        float d = sqrt(max((by * by) - (ay * cy), 0.0));
                        float t1 = (by - d) / ay;
                        float t2 = (by + d) / ay;
                        if (abs(ay) < kQuadraticEpsilon) {
                            t1 = t2 = cy * 0.5 / by;
                        }

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


                segmentIndex += 1;
                numSegs -= 1;
            }

            rayIndex += 1;
            numRays -= 1;
        }


        //debugPrintfEXT("UV: (%f, %f)\n", textureCoord.r, textureCoord.g);
        //bool debug_print = (textureCoord.r > 550.0) && (textureCoord.r < 750.0) && (textureCoord.g > 550.0) && (textureCoord.g < 670.0);
        
        // if (debug_print) {
        //     debugPrintfEXT("DF: %f, %f\n", dF.x, dF.y);
        //     //debugPrintfEXT("DF: %f, %f, %f, %f\n", dFdxCoarse(textureCoord.r), dFdxCoarse(textureCoord.g), dFdyCoarse(textureCoord.r), dFdyCoarse(textureCoord.g));
        // }
        //debugPrintfEXT("Offset|NumSegments: %u: %u | %u\n", glyph, segment_data_offset, segment_data_length);

        // if (debug_print) {
        //     debugPrintfEXT("OI: %u, %u\n", glyph, cnt); //Outline Index
        // }

        // if (debug_print) {
        //     debugPrintfEXT("TC %f\n", coverage);
        // }
        float alpha = clamp(abs(coverage) / avg_div , 0.0, 1.0);
        //Maybe discard in future if alpha is fully transparent!
        //Also check alpha * alpha theory
        preMultipliedAlphaColorOutput = color * alpha;
    }
}

