//Media Enhanced Swiftlet Graphics Rust Library using Vulkan
//MIT License
//Copyright (c) 2024 Jared Loewenthal
//
//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files (the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions:
//
//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.
//
//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.

pub type Word = u32;
const MAGIC_NUMBER: Word = 0x07230203;
const COMPATABLE_VERSION_NUMBER: Word = 0x00010300;
const LATEST_VERSION_NUMBER: Word = 0x00010600;

pub fn create_word_stream_header(use_latest_version: bool, id_bound: Word) -> Vec<Word> {
    let version_number = if use_latest_version {
        LATEST_VERSION_NUMBER
    } else {
        COMPATABLE_VERSION_NUMBER
    };
    vec![MAGIC_NUMBER, version_number, 0, id_bound, 0]
}

pub trait EnablingCapabilities {
    fn get_required_capabilities(&self, capabilities: &mut [Capability]) -> usize;
}

#[derive(Clone, Copy, Debug)]
#[repr(u16)]
pub enum OpcodeName {
    Nop = 0,
    Undef = 1,
    SourceContinued = 2,
    Source = 3,
    SourceExtension = 4,
    Name = 5,
    MemberName = 6,
    String = 7,
    Line = 8,
    Extension = 10,
    ExtInstImport = 11,
    ExtInst = 12,
    MemoryModel = 14,
    EntryPoint = 15,
    ExecutionMode = 16,
    Capability = 17,
    TypeVoid = 19,
    TypeBool = 20,
    TypeInt = 21,
    TypeFloat = 22,
    TypeVector = 23,
    TypeMatrix = 24,
    TypeImage = 25,
    TypeSampler = 26,
    TypeSampledImage = 27,
    TypeArray = 28,
    TypeRuntimeArray = 29,
    TypeStruct = 30,
    TypeOpaque = 31,
    TypePointer = 32,
    TypeFunction = 33,
    TypeEvent = 34,
    TypeDeviceEvent = 35,
    TypeReserveId = 36,
    TypeQueue = 37,
    TypePipe = 38,
    TypeForwardPointer = 39,
    ConstantTrue = 41,
    ConstantFalse = 42,
    Constant = 43,
    ConstantComposite = 44,
    ConstantSampler = 45,
    ConstantNull = 46,
    SpecConstantTrue = 48,
    SpecConstantFalse = 49,
    SpecConstant = 50,
    SpecConstantComposite = 51,
    SpecConstantOp = 52,
    Function = 54,
    FunctionParameter = 55,
    FunctionEnd = 56,
    FunctionCall = 57,
    Variable = 59,
    ImageTexelPointer = 60,
    Load = 61,
    Store = 62,
    CopyMemory = 63,
    CopyMemorySized = 64,
    AccessChain = 65,
    InBoundsAccessChain = 66,
    PtrAccessChain = 67,
    ArrayLength = 68,
    GenericPtrMemSemantics = 69,
    InBoundsPtrAccessChain = 70,
    Decorate = 71,
    MemberDecorate = 72,
    DecorationGroup = 73,
    GroupDecorate = 74,
    GroupMemberDecorate = 75,
    VectorExtractDynamic = 77,
    VectorInsertDynamic = 78,
    VectorShuffle = 79,
    CompositeConstruct = 80,
    CompositeExtract = 81,
    CompositeInsert = 82,
    CopyObject = 83,
    Transpose = 84,
    SampledImage = 86,
    ImageSampleImplicitLod = 87,
    ImageSampleExplicitLod = 88,
    ImageSampleDrefImplicitLod = 89,
    ImageSampleDrefExplicitLod = 90,
    ImageSampleProjImplicitLod = 91,
    ImageSampleProjExplicitLod = 92,
    ImageSampleProjDrefImplicitLod = 93,
    ImageSampleProjDrefExplicitLod = 94,
    ImageFetch = 95,
    ImageGather = 96,
    ImageDrefGather = 97,
    ImageRead = 98,
    ImageWrite = 99,
    Image = 100,
    ImageQueryFormat = 101,
    ImageQueryOrder = 102,
    ImageQuerySizeLod = 103,
    ImageQuerySize = 104,
    ImageQueryLod = 105,
    ImageQueryLevels = 106,
    ImageQuerySamples = 107,
    ConvertFToU = 109,
    ConvertFToS = 110,
    ConvertSToF = 111,
    ConvertUToF = 112,
    UConvert = 113,
    SConvert = 114,
    FConvert = 115,
    QuantizeToF16 = 116,
    ConvertPtrToU = 117,
    SatConvertSToU = 118,
    SatConvertUToS = 119,
    ConvertUToPtr = 120,
    PtrCastToGeneric = 121,
    GenericCastToPtr = 122,
    GenericCastToPtrExplicit = 123,
    Bitcast = 124,
    SNegate = 126,
    FNegate = 127,
    IAdd = 128,
    FAdd = 129,
    ISub = 130,
    FSub = 131,
    IMul = 132,
    FMul = 133,
    UDiv = 134,
    SDiv = 135,
    FDiv = 136,
    UMod = 137,
    SRem = 138,
    SMod = 139,
    FRem = 140,
    FMod = 141,
    VectorTimesScalar = 142,
    MatrixTimesScalar = 143,
    VectorTimesMatrix = 144,
    MatrixTimesVector = 145,
    MatrixTimesMatrix = 146,
    OuterProduct = 147,
    Dot = 148,
    IAddCarry = 149,
    ISubBorrow = 150,
    UMulExtended = 151,
    SMulExtended = 152,
    Any = 154,
    All = 155,
    IsNan = 156,
    IsInf = 157,
    IsFinite = 158,
    IsNormal = 159,
    SignBitSet = 160,
    LessOrGreater = 161,
    Ordered = 162,
    Unordered = 163,
    LogicalEqual = 164,
    LogicalNotEqual = 165,
    LogicalOr = 166,
    LogicalAnd = 167,
    LogicalNot = 168,
    Select = 169,
    IEqual = 170,
    INotEqual = 171,
    UGreaterThan = 172,
    SGreaterThan = 173,
    UGreaterThanEqual = 174,
    SGreaterThanEqual = 175,
    ULessThan = 176,
    SLessThan = 177,
    ULessThanEqual = 178,
    SLessThanEqual = 179,
    FOrdEqual = 180,
    FUnordEqual = 181,
    FOrdNotEqual = 182,
    FUnordNotEqual = 183,
    FOrdLessThan = 184,
    FUnordLessThan = 185,
    FOrdGreaterThan = 186,
    FUnordGreaterThan = 187,
    FOrdLessThanEqual = 188,
    FUnordLessThanEqual = 189,
    FOrdGreaterThanEqual = 190,
    FUnordGreaterThanEqual = 191,
    ShiftRightLogical = 194,
    ShiftRightArithmetic = 195,
    ShiftLeftLogical = 196,
    BitwiseOr = 197,
    BitwiseXor = 198,
    BitwiseAnd = 199,
    Not = 200,
    BitFieldInsert = 201,
    BitFieldSExtract = 202,
    BitFieldUExtract = 203,
    BitReverse = 204,
    BitCount = 205,
    DPdx = 207,
    DPdy = 208,
    Fwidth = 209,
    DPdxFine = 210,
    DPdyFine = 211,
    FwidthFine = 212,
    DPdxCoarse = 213,
    DPdyCoarse = 214,
    FwidthCoarse = 215,
    EmitVertex = 218,
    EndPrimitive = 219,
    EmitStreamVertex = 220,
    EndStreamPrimitive = 221,
    ControlBarrier = 224,
    MemoryBarrier = 225,
    AtomicLoad = 227,
    AtomicStore = 228,
    AtomicExchange = 229,
    AtomicCompareExchange = 230,
    AtomicCompareExchangeWeak = 231,
    AtomicIIncrement = 232,
    AtomicIDecrement = 233,
    AtomicIAdd = 234,
    AtomicISub = 235,
    AtomicSMin = 236,
    AtomicUMin = 237,
    AtomicSMax = 238,
    AtomicUMax = 239,
    AtomicAnd = 240,
    AtomicOr = 241,
    AtomicXor = 242,
    Phi = 245,
    LoopMerge = 246,
    SelectionMerge = 247,
    Label = 248,
    Branch = 249,
    BranchConditional = 250,
    Switch = 251,
    Kill = 252,
    Return = 253,
    ReturnValue = 254,
    Unreachable = 255,
    LifetimeStart = 256,
    LifetimeStop = 257,
    GroupAsyncCopy = 259,
    GroupWaitEvents = 260,
    GroupAll = 261,
    GroupAny = 262,
    GroupBroadcast = 263,
    GroupIAdd = 264,
    GroupFAdd = 265,
    GroupFMin = 266,
    GroupUMin = 267,
    GroupSMin = 268,
    GroupFMax = 269,
    GroupUMax = 270,
    GroupSMax = 271,
    ReadPipe = 274,
    WritePipe = 275,
    ReservedReadPipe = 276,
    ReservedWritePipe = 277,
    ReserveReadPipePackets = 278,
    ReserveWritePipePackets = 279,
    CommitReadPipe = 280,
    CommitWritePipe = 281,
    IsValidReserveId = 282,
    GetNumPipePackets = 283,
    GetMaxPipePackets = 284,
    GroupReserveReadPipePackets = 285,
    GroupReserveWritePipePackets = 286,
    GroupCommitReadPipe = 287,
    GroupCommitWritePipe = 288,
    EnqueueMarker = 291,
    EnqueueKernel = 292,
    GetKernelNDrangeSubGroupCount = 293,
    GetKernelNDrangeMaxSubGroupSize = 294,
    GetKernelWorkGroupSize = 295,
    GetKernelPreferredWorkGroupSizeMultiple = 296,
    RetainEvent = 297,
    ReleaseEvent = 298,
    CreateUserEvent = 299,
    IsValidEvent = 300,
    SetUserEventStatus = 301,
    CaptureEventProfilingInfo = 302,
    GetDefaultQueue = 303,
    BuildNDRange = 304,
    ImageSparseSampleImplicitLod = 305,
    ImageSparseSampleExplicitLod = 306,
    ImageSparseSampleDrefImplicitLod = 307,
    ImageSparseSampleDrefExplicitLod = 308,
    ImageSparseSampleProjImplicitLod = 309,
    ImageSparseSampleProjExplicitLod = 310,
    ImageSparseSampleProjDrefImplicitLod = 311,
    ImageSparseSampleProjDrefExplicitLod = 312,
    ImageSparseFetch = 313,
    ImageSparseGather = 314,
    ImageSparseDrefGather = 315,
    ImageSparseTexelsResident = 316,
    NoLine = 317,
    AtomicFlagTestAndSet = 318,
    AtomicFlagClear = 319,
    ImageSparseRead = 320,
    SizeOf = 321,
    TypePipeStorage = 322,
    ConstantPipeStorage = 323,
    CreatePipeFromPipeStorage = 324,
    GetKernelLocalSizeForSubgroupCount = 325,
    GetKernelMaxNumSubgroups = 326,
    TypeNamedBarrier = 327,
    NamedBarrierInitialize = 328,
    MemoryNamedBarrier = 329,
    ModuleProcessed = 330,
    ExecutionModeId = 331,
    DecorateId = 332,
    GroupNonUniformElect = 333,
    GroupNonUniformAll = 334,
    GroupNonUniformAny = 335,
    GroupNonUniformAllEqual = 336,
    GroupNonUniformBroadcast = 337,
    GroupNonUniformBroadcastFirst = 338,
    GroupNonUniformBallot = 339,
    GroupNonUniformInverseBallot = 340,
    GroupNonUniformBallotBitExtract = 341,
    GroupNonUniformBallotBitCount = 342,
    GroupNonUniformBallotFindLSB = 343,
    GroupNonUniformBallotFindMSB = 344,
    GroupNonUniformShuffle = 345,
    GroupNonUniformShuffleXor = 346,
    GroupNonUniformShuffleUp = 347,
    GroupNonUniformShuffleDown = 348,
    GroupNonUniformIAdd = 349,
    GroupNonUniformFAdd = 350,
    GroupNonUniformIMul = 351,
    GroupNonUniformFMul = 352,
    GroupNonUniformSMin = 353,
    GroupNonUniformUMin = 354,
    GroupNonUniformFMin = 355,
    GroupNonUniformSMax = 356,
    GroupNonUniformUMax = 357,
    GroupNonUniformFMax = 358,
    GroupNonUniformBitwiseAnd = 359,
    GroupNonUniformBitwiseOr = 360,
    GroupNonUniformBitwiseXor = 361,
    GroupNonUniformLogicalAnd = 362,
    GroupNonUniformLogicalOr = 363,
    GroupNonUniformLogicalXor = 364,
    GroupNonUniformQuadBroadcast = 365,
    GroupNonUniformQuadSwap = 366,
    CopyLogical = 400,
    PtrEqual = 401,
    PtrNotEqual = 402,
    PtrDiff = 403,
    ColorAttachmentReadEXT = 4_160,
    DepthAttachmentReadEXT = 4_161,
    StencilAttachmentReadEXT = 4_162,
    TerminateInvocation = 4_416,
    SubgroupBallotKHR = 4_421,
    SubgroupFirstInvocationKHR = 4_422,
    SubgroupAllKHR = 4_428,
    SubgroupAnyKHR = 4_429,
    SubgroupAllEqualKHR = 4_430,
    GroupNonUniformRotateKHR = 4_431,
    SubgroupReadInvocationKHR = 4_432,
    TraceRayKHR = 4_445,
    ExecuteCallableKHR = 4_446,
    ConvertUToAccelerationStructureKHR = 4_447,
    IgnoreIntersectionKHR = 4_448,
    TerminateRayKHR = 4_449,
    SDot = 4_450,
    UDot = 4_451,
    SUDot = 4_452,
    SDotAccSat = 4_453,
    UDotAccSat = 4_454,
    SUDotAccSat = 4_455,
    TypeCooperativeMatrixKHR = 4_456,
    CooperativeMatrixLoadKHR = 4_457,
    CooperativeMatrixStoreKHR = 4_458,
    CooperativeMatrixMulAddKHR = 4_459,
    CooperativeMatrixLengthKHR = 4_460,
    TypeRayQueryKHR = 4_472,
    RayQueryInitializeKHR = 4_473,
    RayQueryTerminateKHR = 4_474,
    RayQueryGenerateIntersectionKHR = 4_475,
    RayQueryConfirmIntersectionKHR = 4_476,
    RayQueryProceedKHR = 4_477,
    RayQueryGetIntersectionTypeKHR = 4_479,
    ImageSampleWeightedQCOM = 4_480,
    ImageBoxFilterQCOM = 4_481,
    ImageBlockMatchSSDQCOM = 4_482,
    ImageBlockMatchSADQCOM = 4_483,
    GroupIAddNonUniformAMD = 5_000,
    GroupFAddNonUniformAMD = 5_001,
    GroupFMinNonUniformAMD = 5_002,
    GroupUMinNonUniformAMD = 5_003,
    GroupSMinNonUniformAMD = 5_004,
    GroupFMaxNonUniformAMD = 5_005,
    GroupUMaxNonUniformAMD = 5_006,
    GroupSMaxNonUniformAMD = 5_007,
    FragmentMaskFetchAMD = 5_011,
    FragmentFetchAMD = 5_012,
    ReadClockKHR = 5_056,
    FinalizeNodePayloadsAMDX = 5_075,
    FinishWritingNodePayloadAMDX = 5_078,
    InitializeNodePayloadsAMDX = 5_090,
    HitObjectRecordHitMotionNV = 5_249,
    HitObjectRecordHitWithIndexMotionNV = 5_250,
    HitObjectRecordMissMotionNV = 5_251,
    HitObjectGetWorldToObjectNV = 5_252,
    HitObjectGetObjectToWorldNV = 5_253,
    HitObjectGetObjectRayDirectionNV = 5_254,
    HitObjectGetObjectRayOriginNV = 5_255,
    HitObjectTraceRayMotionNV = 5_256,
    HitObjectGetShaderRecordBufferHandleNV = 5_257,
    HitObjectGetShaderBindingTableRecordIndexNV = 5_258,
    HitObjectRecordEmptyNV = 5_259,
    HitObjectTraceRayNV = 5_260,
    HitObjectRecordHitNV = 5_261,
    HitObjectRecordHitWithIndexNV = 5_262,
    HitObjectRecordMissNV = 5_263,
    HitObjectExecuteShaderNV = 5_264,
    HitObjectGetCurrentTimeNV = 5_265,
    HitObjectGetAttributesNV = 5_266,
    HitObjectGetHitKindNV = 5_267,
    HitObjectGetPrimitiveIndexNV = 5_268,
    HitObjectGetGeometryIndexNV = 5_269,
    HitObjectGetInstanceIdNV = 5_270,
    HitObjectGetInstanceCustomIndexNV = 5_271,
    HitObjectGetWorldRayDirectionNV = 5_272,
    HitObjectGetWorldRayOriginNV = 5_273,
    HitObjectGetRayTMaxNV = 5_274,
    HitObjectGetRayTMinNV = 5_275,
    HitObjectIsEmptyNV = 5_276,
    HitObjectIsHitNV = 5_277,
    HitObjectIsMissNV = 5_278,
    ReorderThreadWithHitObjectNV = 5_279,
    ReorderThreadWithHintNV = 5_280,
    TypeHitObjectNV = 5_281,
    ImageSampleFootprintNV = 5_283,
    EmitMeshTasksEXT = 5_294,
    SetMeshOutputsEXT = 5_295,
    GroupNonUniformPartitionNV = 5_296,
    WritePackedPrimitiveIndices4x8NV = 5_299,
    FetchMicroTriangleVertexPositionNV = 5_300,
    FetchMicroTriangleVertexBarycentricNV = 5_301,
    ReportIntersectionKHR = 5_334,
    IgnoreIntersectionNV = 5_335,
    TerminateRayNV = 5_336,
    TraceNV = 5_337,
    TraceMotionNV = 5_338,
    TraceRayMotionNV = 5_339,
    RayQueryGetIntersectionTriangleVertexPositionsKHR = 5_340,
    TypeAccelerationStructureKHR = 5_341,
    ExecuteCallableNV = 5_344,
    TypeCooperativeMatrixNV = 5_358,
    CooperativeMatrixLoadNV = 5_359,
    CooperativeMatrixStoreNV = 5_360,
    CooperativeMatrixMulAddNV = 5_361,
    CooperativeMatrixLengthNV = 5_362,
    BeginInvocationInterlockEXT = 5_364,
    EndInvocationInterlockEXT = 5_365,
    DemoteToHelperInvocation = 5_380,
    IsHelperInvocationEXT = 5_381,
    ConvertUToImageNV = 5_391,
    ConvertUToSamplerNV = 5_392,
    ConvertImageToUNV = 5_393,
    ConvertSamplerToUNV = 5_394,
    ConvertUToSampledImageNV = 5_395,
    ConvertSampledImageToUNV = 5_396,
    SamplerImageAddressingModeNV = 5_397,
    SubgroupShuffleINTEL = 5_571,
    SubgroupShuffleDownINTEL = 5_572,
    SubgroupShuffleUpINTEL = 5_573,
    SubgroupShuffleXorINTEL = 5_574,
    SubgroupBlockReadINTEL = 5_575,
    SubgroupBlockWriteINTEL = 5_576,
    SubgroupImageBlockReadINTEL = 5_577,
    SubgroupImageBlockWriteINTEL = 5_578,
    SubgroupImageMediaBlockReadINTEL = 5_580,
    SubgroupImageMediaBlockWriteINTEL = 5_581,
    UCountLeadingZerosINTEL = 5_585,
    UCountTrailingZerosINTEL = 5_586,
    AbsISubINTEL = 5_587,
    AbsUSubINTEL = 5_588,
    IAddSatINTEL = 5_589,
    UAddSatINTEL = 5_590,
    IAverageINTEL = 5_591,
    UAverageINTEL = 5_592,
    IAverageRoundedINTEL = 5_593,
    UAverageRoundedINTEL = 5_594,
    ISubSatINTEL = 5_595,
    USubSatINTEL = 5_596,
    IMul32x16INTEL = 5_597,
    UMul32x16INTEL = 5_598,
    ConstantFunctionPointerINTEL = 5_600,
    FunctionPointerCallINTEL = 5_601,
    AsmTargetINTEL = 5_609,
    AsmINTEL = 5_610,
    AsmCallINTEL = 5_611,
    AtomicFMinEXT = 5_614,
    AtomicFMaxEXT = 5_615,
    AssumeTrueKHR = 5_630,
    ExpectKHR = 5_631,
    DecorateString = 5_632,
    MemberDecorateString = 5_633,
    VmeImageINTEL = 5_699,
    TypeVmeImageINTEL = 5_700,
    TypeAvcImePayloadINTEL = 5_701,
    TypeAvcRefPayloadINTEL = 5_702,
    TypeAvcSicPayloadINTEL = 5_703,
    TypeAvcMcePayloadINTEL = 5_704,
    TypeAvcMceResultINTEL = 5_705,
    TypeAvcImeResultINTEL = 5_706,
    TypeAvcImeResultSingleReferenceStreamoutINTEL = 5_707,
    TypeAvcImeResultDualReferenceStreamoutINTEL = 5_708,
    TypeAvcImeSingleReferenceStreaminINTEL = 5_709,
    TypeAvcImeDualReferenceStreaminINTEL = 5_710,
    TypeAvcRefResultINTEL = 5_711,
    TypeAvcSicResultINTEL = 5_712,
    SubgroupAvcMceGetDefaultInterBaseMultiReferencePenaltyINTEL = 5_713,
    SubgroupAvcMceSetInterBaseMultiReferencePenaltyINTEL = 5_714,
    SubgroupAvcMceGetDefaultInterShapePenaltyINTEL = 5_715,
    SubgroupAvcMceSetInterShapePenaltyINTEL = 5_716,
    SubgroupAvcMceGetDefaultInterDirectionPenaltyINTEL = 5_717,
    SubgroupAvcMceSetInterDirectionPenaltyINTEL = 5_718,
    SubgroupAvcMceGetDefaultIntraLumaShapePenaltyINTEL = 5_719,
    SubgroupAvcMceGetDefaultInterMotionVectorCostTableINTEL = 5_720,
    SubgroupAvcMceGetDefaultHighPenaltyCostTableINTEL = 5_721,
    SubgroupAvcMceGetDefaultMediumPenaltyCostTableINTEL = 5_722,
    SubgroupAvcMceGetDefaultLowPenaltyCostTableINTEL = 5_723,
    SubgroupAvcMceSetMotionVectorCostFunctionINTEL = 5_724,
    SubgroupAvcMceGetDefaultIntraLumaModePenaltyINTEL = 5_725,
    SubgroupAvcMceGetDefaultNonDcLumaIntraPenaltyINTEL = 5_726,
    SubgroupAvcMceGetDefaultIntraChromaModeBasePenaltyINTEL = 5_727,
    SubgroupAvcMceSetAcOnlyHaarINTEL = 5_728,
    SubgroupAvcMceSetSourceInterlacedFieldPolarityINTEL = 5_729,
    SubgroupAvcMceSetSingleReferenceInterlacedFieldPolarityINTEL = 5_730,
    SubgroupAvcMceSetDualReferenceInterlacedFieldPolaritiesINTEL = 5_731,
    SubgroupAvcMceConvertToImePayloadINTEL = 5_732,
    SubgroupAvcMceConvertToImeResultINTEL = 5_733,
    SubgroupAvcMceConvertToRefPayloadINTEL = 5_734,
    SubgroupAvcMceConvertToRefResultINTEL = 5_735,
    SubgroupAvcMceConvertToSicPayloadINTEL = 5_736,
    SubgroupAvcMceConvertToSicResultINTEL = 5_737,
    SubgroupAvcMceGetMotionVectorsINTEL = 5_738,
    SubgroupAvcMceGetInterDistortionsINTEL = 5_739,
    SubgroupAvcMceGetBestInterDistortionsINTEL = 5_740,
    SubgroupAvcMceGetInterMajorShapeINTEL = 5_741,
    SubgroupAvcMceGetInterMinorShapeINTEL = 5_742,
    SubgroupAvcMceGetInterDirectionsINTEL = 5_743,
    SubgroupAvcMceGetInterMotionVectorCountINTEL = 5_744,
    SubgroupAvcMceGetInterReferenceIdsINTEL = 5_745,
    SubgroupAvcMceGetInterReferenceInterlacedFieldPolaritiesINTEL = 5_746,
    SubgroupAvcImeInitializeINTEL = 5_747,
    SubgroupAvcImeSetSingleReferenceINTEL = 5_748,
    SubgroupAvcImeSetDualReferenceINTEL = 5_749,
    SubgroupAvcImeRefWindowSizeINTEL = 5_750,
    SubgroupAvcImeAdjustRefOffsetINTEL = 5_751,
    SubgroupAvcImeConvertToMcePayloadINTEL = 5_752,
    SubgroupAvcImeSetMaxMotionVectorCountINTEL = 5_753,
    SubgroupAvcImeSetUnidirectionalMixDisableINTEL = 5_754,
    SubgroupAvcImeSetEarlySearchTerminationThresholdINTEL = 5_755,
    SubgroupAvcImeSetWeightedSadINTEL = 5_756,
    SubgroupAvcImeEvaluateWithSingleReferenceINTEL = 5_757,
    SubgroupAvcImeEvaluateWithDualReferenceINTEL = 5_758,
    SubgroupAvcImeEvaluateWithSingleReferenceStreaminINTEL = 5_759,
    SubgroupAvcImeEvaluateWithDualReferenceStreaminINTEL = 5_760,
    SubgroupAvcImeEvaluateWithSingleReferenceStreamoutINTEL = 5_761,
    SubgroupAvcImeEvaluateWithDualReferenceStreamoutINTEL = 5_762,
    SubgroupAvcImeEvaluateWithSingleReferenceStreaminoutINTEL = 5_763,
    SubgroupAvcImeEvaluateWithDualReferenceStreaminoutINTEL = 5_764,
    SubgroupAvcImeConvertToMceResultINTEL = 5_765,
    SubgroupAvcImeGetSingleReferenceStreaminINTEL = 5_766,
    SubgroupAvcImeGetDualReferenceStreaminINTEL = 5_767,
    SubgroupAvcImeStripSingleReferenceStreamoutINTEL = 5_768,
    SubgroupAvcImeStripDualReferenceStreamoutINTEL = 5_769,
    SubgroupAvcImeGetStreamoutSingleReferenceMajorShapeMotionVectorsINTEL = 5_770,
    SubgroupAvcImeGetStreamoutSingleReferenceMajorShapeDistortionsINTEL = 5_771,
    SubgroupAvcImeGetStreamoutSingleReferenceMajorShapeReferenceIdsINTEL = 5_772,
    SubgroupAvcImeGetStreamoutDualReferenceMajorShapeMotionVectorsINTEL = 5_773,
    SubgroupAvcImeGetStreamoutDualReferenceMajorShapeDistortionsINTEL = 5_774,
    SubgroupAvcImeGetStreamoutDualReferenceMajorShapeReferenceIdsINTEL = 5_775,
    SubgroupAvcImeGetBorderReachedINTEL = 5_776,
    SubgroupAvcImeGetTruncatedSearchIndicationINTEL = 5_777,
    SubgroupAvcImeGetUnidirectionalEarlySearchTerminationINTEL = 5_778,
    SubgroupAvcImeGetWeightingPatternMinimumMotionVectorINTEL = 5_779,
    SubgroupAvcImeGetWeightingPatternMinimumDistortionINTEL = 5_780,
    SubgroupAvcFmeInitializeINTEL = 5_781,
    SubgroupAvcBmeInitializeINTEL = 5_782,
    SubgroupAvcRefConvertToMcePayloadINTEL = 5_783,
    SubgroupAvcRefSetBidirectionalMixDisableINTEL = 5_784,
    SubgroupAvcRefSetBilinearFilterEnableINTEL = 5_785,
    SubgroupAvcRefEvaluateWithSingleReferenceINTEL = 5_786,
    SubgroupAvcRefEvaluateWithDualReferenceINTEL = 5_787,
    SubgroupAvcRefEvaluateWithMultiReferenceINTEL = 5_788,
    SubgroupAvcRefEvaluateWithMultiReferenceInterlacedINTEL = 5_789,
    SubgroupAvcRefConvertToMceResultINTEL = 5_790,
    SubgroupAvcSicInitializeINTEL = 5_791,
    SubgroupAvcSicConfigureSkcINTEL = 5_792,
    SubgroupAvcSicConfigureIpeLumaINTEL = 5_793,
    SubgroupAvcSicConfigureIpeLumaChromaINTEL = 5_794,
    SubgroupAvcSicGetMotionVectorMaskINTEL = 5_795,
    SubgroupAvcSicConvertToMcePayloadINTEL = 5_796,
    SubgroupAvcSicSetIntraLumaShapePenaltyINTEL = 5_797,
    SubgroupAvcSicSetIntraLumaModeCostFunctionINTEL = 5_798,
    SubgroupAvcSicSetIntraChromaModeCostFunctionINTEL = 5_799,
    SubgroupAvcSicSetBilinearFilterEnableINTEL = 5_800,
    SubgroupAvcSicSetSkcForwardTransformEnableINTEL = 5_801,
    SubgroupAvcSicSetBlockBasedRawSkipSadINTEL = 5_802,
    SubgroupAvcSicEvaluateIpeINTEL = 5_803,
    SubgroupAvcSicEvaluateWithSingleReferenceINTEL = 5_804,
    SubgroupAvcSicEvaluateWithDualReferenceINTEL = 5_805,
    SubgroupAvcSicEvaluateWithMultiReferenceINTEL = 5_806,
    SubgroupAvcSicEvaluateWithMultiReferenceInterlacedINTEL = 5_807,
    SubgroupAvcSicConvertToMceResultINTEL = 5_808,
    SubgroupAvcSicGetIpeLumaShapeINTEL = 5_809,
    SubgroupAvcSicGetBestIpeLumaDistortionINTEL = 5_810,
    SubgroupAvcSicGetBestIpeChromaDistortionINTEL = 5_811,
    SubgroupAvcSicGetPackedIpeLumaModesINTEL = 5_812,
    SubgroupAvcSicGetIpeChromaModeINTEL = 5_813,
    SubgroupAvcSicGetPackedSkcLumaCountThresholdINTEL = 5_814,
    SubgroupAvcSicGetPackedSkcLumaSumThresholdINTEL = 5_815,
    SubgroupAvcSicGetInterRawSadsINTEL = 5_816,
    VariableLengthArrayINTEL = 5_818,
    SaveMemoryINTEL = 5_819,
    RestoreMemoryINTEL = 5_820,
    ArbitraryFloatSinCosPiINTEL = 5_840,
    ArbitraryFloatCastINTEL = 5_841,
    ArbitraryFloatCastFromIntINTEL = 5_842,
    ArbitraryFloatCastToIntINTEL = 5_843,
    ArbitraryFloatAddINTEL = 5_846,
    ArbitraryFloatSubINTEL = 5_847,
    ArbitraryFloatMulINTEL = 5_848,
    ArbitraryFloatDivINTEL = 5_849,
    ArbitraryFloatGTINTEL = 5_850,
    ArbitraryFloatGEINTEL = 5_851,
    ArbitraryFloatLTINTEL = 5_852,
    ArbitraryFloatLEINTEL = 5_853,
    ArbitraryFloatEQINTEL = 5_854,
    ArbitraryFloatRecipINTEL = 5_855,
    ArbitraryFloatRSqrtINTEL = 5_856,
    ArbitraryFloatCbrtINTEL = 5_857,
    ArbitraryFloatHypotINTEL = 5_858,
    ArbitraryFloatSqrtINTEL = 5_859,
    ArbitraryFloatLogINTEL = 5_860,
    ArbitraryFloatLog2INTEL = 5_861,
    ArbitraryFloatLog10INTEL = 5_862,
    ArbitraryFloatLog1pINTEL = 5_863,
    ArbitraryFloatExpINTEL = 5_864,
    ArbitraryFloatExp2INTEL = 5_865,
    ArbitraryFloatExp10INTEL = 5_866,
    ArbitraryFloatExpm1INTEL = 5_867,
    ArbitraryFloatSinINTEL = 5_868,
    ArbitraryFloatCosINTEL = 5_869,
    ArbitraryFloatSinCosINTEL = 5_870,
    ArbitraryFloatSinPiINTEL = 5_871,
    ArbitraryFloatCosPiINTEL = 5_872,
    ArbitraryFloatASinINTEL = 5_873,
    ArbitraryFloatASinPiINTEL = 5_874,
    ArbitraryFloatACosINTEL = 5_875,
    ArbitraryFloatACosPiINTEL = 5_876,
    ArbitraryFloatATanINTEL = 5_877,
    ArbitraryFloatATanPiINTEL = 5_878,
    ArbitraryFloatATan2INTEL = 5_879,
    ArbitraryFloatPowINTEL = 5_880,
    ArbitraryFloatPowRINTEL = 5_881,
    ArbitraryFloatPowNINTEL = 5_882,
    LoopControlINTEL = 5_887,
    AliasDomainDeclINTEL = 5_911,
    AliasScopeDeclINTEL = 5_912,
    AliasScopeListDeclINTEL = 5_913,
    FixedSqrtINTEL = 5_923,
    FixedRecipINTEL = 5_924,
    FixedRsqrtINTEL = 5_925,
    FixedSinINTEL = 5_926,
    FixedCosINTEL = 5_927,
    FixedSinCosINTEL = 5_928,
    FixedSinPiINTEL = 5_929,
    FixedCosPiINTEL = 5_930,
    FixedSinCosPiINTEL = 5_931,
    FixedLogINTEL = 5_932,
    FixedExpINTEL = 5_933,
    PtrCastToCrossWorkgroupINTEL = 5_934,
    CrossWorkgroupCastToPtrINTEL = 5_938,
    ReadPipeBlockingINTEL = 5_946,
    WritePipeBlockingINTEL = 5_947,
    FPGARegINTEL = 5_949,
    RayQueryGetRayTMinKHR = 6_016,
    RayQueryGetRayFlagsKHR = 6_017,
    RayQueryGetIntersectionTKHR = 6_018,
    RayQueryGetIntersectionInstanceCustomIndexKHR = 6_019,
    RayQueryGetIntersectionInstanceIdKHR = 6_020,
    RayQueryGetIntersectionInstanceShaderBindingTableRecordOffsetKHR = 6_021,
    RayQueryGetIntersectionGeometryIndexKHR = 6_022,
    RayQueryGetIntersectionPrimitiveIndexKHR = 6_023,
    RayQueryGetIntersectionBarycentricsKHR = 6_024,
    RayQueryGetIntersectionFrontFaceKHR = 6_025,
    RayQueryGetIntersectionCandidateAABBOpaqueKHR = 6_026,
    RayQueryGetIntersectionObjectRayDirectionKHR = 6_027,
    RayQueryGetIntersectionObjectRayOriginKHR = 6_028,
    RayQueryGetWorldRayDirectionKHR = 6_029,
    RayQueryGetWorldRayOriginKHR = 6_030,
    RayQueryGetIntersectionObjectToWorldKHR = 6_031,
    RayQueryGetIntersectionWorldToObjectKHR = 6_032,
    AtomicFAddEXT = 6_035,
    TypeBufferSurfaceINTEL = 6_086,
    TypeStructContinuedINTEL = 6_090,
    ConstantCompositeContinuedINTEL = 6_091,
    SpecConstantCompositeContinuedINTEL = 6_092,
    ConvertFToBF16INTEL = 6_116,
    ConvertBF16ToFINTEL = 6_117,
    ControlBarrierArriveINTEL = 6_142,
    ControlBarrierWaitINTEL = 6_143,
    GroupIMulKHR = 6_401,
    GroupFMulKHR = 6_402,
    GroupBitwiseAndKHR = 6_403,
    GroupBitwiseOrKHR = 6_404,
    GroupBitwiseXorKHR = 6_405,
    GroupLogicalAndKHR = 6_406,
    GroupLogicalOrKHR = 6_407,
    GroupLogicalXorKHR = 6_408,
}
impl OpcodeName {
    #[inline]
    fn get_opcode(&self) -> u16 {
        *self as u16
    }

    #[inline]
    fn get_word_count(&self) -> (u16, bool) {
        match self {
            Self::Capability => (2, false),
            Self::Extension => (2, true),
            Self::ExtInstImport => (3, true),
            Self::MemoryModel => (3, false),
            Self::EntryPoint => (4, true),
            Self::ExecutionMode => (3, true),
            Self::TypeVoid => (2, false),
            Self::TypeBool => (2, false),
            Self::TypeInt => (4, false),
            Self::TypeFloat => (3, false),
            Self::TypeVector => (4, false),
            Self::TypeMatrix => (4, false),
            Self::TypeArray => (4, false),
            Self::TypeRuntimeArray => (3, false),
            Self::TypePointer => (4, false),
            Self::TypeFunction => (3, true),
            Self::ConstantTrue => (3, false),
            Self::ConstantFalse => (3, false),
            Self::Constant => (4, true),
            Self::Variable => (4, true),
            Self::Function => (5, false),
            Self::FunctionParameter => (3, false),
            Self::FunctionEnd => (1, false),
            _ => panic!("{:?} is not currently supported!", self),
        }
    }

    pub fn get_fixed_word0(&self) -> Word {
        let opcode = self.get_opcode() as Word;
        let (word_count, is_variable) = self.get_word_count();
        if !is_variable {
            opcode | ((word_count as Word) << 16)
        } else {
            panic!("{:?} does not have a fixed word0!", self);
        }
    }

    pub fn get_word0(&self, additional_arguments: u16) -> Word {
        let opcode = self.get_opcode() as Word;
        let (word_count, is_variable) = self.get_word_count();
        if !is_variable && (additional_arguments == 0) {
            opcode | ((word_count as Word) << 16)
        } else if is_variable {
            opcode | (((word_count + additional_arguments) as Word) << 16)
        } else {
            panic!("{:?} is not variable!", self);
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum Capability {
    Matrix = 0,
    Shader = 1,
    Geometry,
    Tessellation,
    Addresses,
    Linkage,
    Kernel,
    Vector16,
    Float16Buffer,
    Float16,
    Float64,
    Int64,
    Int64Atomics,
    ImageBasic,
    ImageReadWrite,
    ImageMipmap,

    Pipes = 17,
    Groups = 18,
    DeviceEnqueue,
    LiteralSampler,
    AtomicStorage,
    Int16,
    TessellationPointSize,
    GeometryPointSize,
    ImageGatherExtended,

    StorageImageMultisample = 27,
    UniformBufferArrayDynamicIndexing = 28,
    SampledImageArrayDynamicIndexing,
    StorageBufferArrayDynamicIndexing,
    StorageImageArrayDynamicIndexing,
    ClipDistance,
    CullDistance,
    ImageCubeArray,
    SampleRateShading,
    ImageRect,
    SampledRect,
    GenericPointer,
    Int8,
    InputAttachment,
    SparseResidency,
    MinLod,
    Sampled1D,
    Image1D,
    SampledCubeArray,
    SampledBuffer,
    ImageBuffer,
    ImageMSArray,
    StorageImageExtendedFormats,
    ImageQuery,
    DerivativeControl,
    InterpolationFunction,
    TransformFeedback,
    GeometryStreams,
    StorageImageReadWithoutFormat,
    StorageImageWriteWithoutFormat,
    MultiViewport,
    SubgroupDispatch,
    NamedBarrier,
    PipeStorage,
    //More can be added in future
}
impl Capability {
    pub fn get_implicit(&self) -> Option<Self> {
        match self {
            Self::Shader => Some(Self::Matrix),
            Self::Geometry => Some(Self::Shader),
            Self::Tessellation => Some(Self::Shader),
            Self::Vector16 => Some(Self::Kernel),
            Self::Float16Buffer => Some(Self::Kernel),
            Self::Int64Atomics => Some(Self::Int64),
            Self::ImageBasic => Some(Self::Kernel),
            Self::ImageReadWrite => Some(Self::ImageBasic),
            Self::ImageMipmap => Some(Self::ImageBasic),
            Self::Pipes => Some(Self::Kernel),
            Self::DeviceEnqueue => Some(Self::Kernel),
            Self::LiteralSampler => Some(Self::Kernel),

            Self::AtomicStorage => Some(Self::Shader),
            Self::TessellationPointSize => Some(Self::Tessellation),
            Self::GeometryPointSize => Some(Self::Geometry),
            Self::ImageGatherExtended => Some(Self::Shader),
            Self::StorageImageMultisample => Some(Self::Shader),
            Self::UniformBufferArrayDynamicIndexing => Some(Self::Shader),
            Self::SampledImageArrayDynamicIndexing => Some(Self::Shader),
            Self::StorageBufferArrayDynamicIndexing => Some(Self::Shader),
            Self::StorageImageArrayDynamicIndexing => Some(Self::Shader),
            Self::ClipDistance => Some(Self::Shader),
            Self::CullDistance => Some(Self::Shader),
            Self::ImageCubeArray => Some(Self::SampledCubeArray),

            Self::SampleRateShading => Some(Self::Shader),
            Self::ImageRect => Some(Self::SampledRect),
            Self::SampledRect => Some(Self::Shader),
            Self::GenericPointer => Some(Self::Addresses),
            Self::InputAttachment => Some(Self::Shader),
            Self::SparseResidency => Some(Self::Shader),
            Self::MinLod => Some(Self::Shader),

            Self::Image1D => Some(Self::Sampled1D),
            Self::SampledCubeArray => Some(Self::Shader),
            Self::ImageBuffer => Some(Self::SampledBuffer),
            Self::ImageMSArray => Some(Self::Shader),
            Self::StorageImageExtendedFormats => Some(Self::Shader),
            Self::ImageQuery => Some(Self::Shader),
            Self::DerivativeControl => Some(Self::Shader),

            Self::InterpolationFunction => Some(Self::Shader),
            Self::TransformFeedback => Some(Self::Shader),
            Self::GeometryStreams => Some(Self::Geometry),
            Self::StorageImageReadWithoutFormat => Some(Self::Shader),
            Self::StorageImageWriteWithoutFormat => Some(Self::Shader),

            Self::MultiViewport => Some(Self::Geometry),
            Self::SubgroupDispatch => Some(Self::DeviceEnqueue),
            Self::NamedBarrier => Some(Self::Kernel),
            Self::PipeStorage => Some(Self::Pipes),

            _ => None,
        }
    }
}

#[inline]
pub fn get_str_word_count(name: &str) -> u16 {
    let num_bytes = name.as_bytes().len() + 1;
    let word_count = (num_bytes + 3) >> 2;
    word_count as u16
}

pub fn add_str_bytes(str_bytes: &[u8], data: &mut Vec<Word>) {
    let str_len = str_bytes.len();
    for ind in (0..str_len).step_by(4) {
        let word = ((str_bytes[ind + 3] as u32) << 24)
            | ((str_bytes[ind + 2] as u32) << 16)
            | ((str_bytes[ind + 1] as u32) << 8)
            | (str_bytes[ind] as u32);
        data.push(word);
    }
    let final_word_ind = str_len & !0x3;
    let final_word = match str_len - final_word_ind {
        3 => {
            ((str_bytes[final_word_ind + 2] as u32) << 16)
                | ((str_bytes[final_word_ind + 1] as u32) << 8)
                | (str_bytes[final_word_ind] as u32)
        }
        2 => ((str_bytes[final_word_ind + 1] as u32) << 8) | (str_bytes[final_word_ind] as u32),
        1 => str_bytes[final_word_ind] as u32,
        0 => 0,
        _ => panic!("Add str data issue: {}", str_len - final_word_ind),
    };
    data.push(final_word);
}

pub fn add_str_data(name: &str, data: &mut Vec<Word>) {
    add_str_bytes(name.as_bytes(), data);
}

pub enum AddressingModel {
    Logical,
    Physical32,
    Physical64,
    //PhysicalStorageBuffer64 = 5348,
}
impl AddressingModel {
    pub fn get_word(&self) -> Word {
        match self {
            Self::Logical => 0,
            Self::Physical32 => 1,
            Self::Physical64 => 2,
        }
    }
}

impl EnablingCapabilities for AddressingModel {
    fn get_required_capabilities(&self, capabilities: &mut [Capability]) -> usize {
        match self {
            Self::Physical32 => {
                capabilities[0] = Capability::Addresses;
                1
            }
            Self::Physical64 => {
                capabilities[0] = Capability::Addresses;
                1
            }
            //Self::PhysicalStorageBuffer64 => Capability::PhysicalStorageBufferAddresses,
            _ => 0,
        }
    }
}

pub enum MemoryModel {
    GLSL450,
    OpenCL,
    //Vulkan = 3,
}
impl MemoryModel {
    pub fn get_word(&self) -> Word {
        match self {
            Self::GLSL450 => 1,
            Self::OpenCL => 2,
        }
    }
}

impl EnablingCapabilities for MemoryModel {
    fn get_required_capabilities(&self, capabilities: &mut [Capability]) -> usize {
        match self {
            Self::GLSL450 => {
                capabilities[0] = Capability::Shader;
                1
            }
            Self::OpenCL => {
                capabilities[0] = Capability::Kernel;
                1
            } //Self::Vulkan => Capability::VulkanMemoryModel,
        }
    }
}

pub enum FunctionControl {
    None,
    Inline,
    DontInline,
    Pure,
    Const,
    InlinePure,
    DontInlinePure,
    InlineConst,
    DontInlineConst,
}
impl FunctionControl {
    pub fn get_word(&self) -> Word {
        match self {
            Self::None => 0x0,
            Self::Inline => 0x1,
            Self::DontInline => 0x2,
            Self::Pure => 0x4,
            Self::Const => 0x8,
            Self::InlinePure => 0x5,
            Self::DontInlinePure => 0x6,
            Self::InlineConst => 0x9,
            Self::DontInlineConst => 0xA,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum StorageClass {
    UniformConstant,
    Input,
    Uniform,
    Output,
    Workgroup,
    CrossWorkgroup,
    //Private,
    Function,
    Generic,
    PushConstant,
    AtomicCounter,
    Image,
    StorageBuffer,
}
impl StorageClass {
    pub fn get_word(&self) -> Word {
        match self {
            Self::UniformConstant => 0,
            Self::Input => 1,
            Self::Uniform => 2,
            Self::Output => 3,
            Self::Workgroup => 4,
            Self::CrossWorkgroup => 5,
            //Self::Private => 6,
            Self::Function => 7,
            Self::Generic => 8,
            Self::PushConstant => 9,
            Self::AtomicCounter => 10,
            Self::Image => 11,
            Self::StorageBuffer => 12,
        }
    }
}

impl EnablingCapabilities for StorageClass {
    fn get_required_capabilities(&self, capabilities: &mut [Capability]) -> usize {
        match self {
            Self::Uniform => {
                capabilities[0] = Capability::Shader;
                1
            }
            Self::Output => {
                capabilities[0] = Capability::Shader;
                1
            }
            // Self::Private => {
            //     capabilities[0] = Capability::Shader;
            //     capabilities[1] = Capability::VectorComputeINTEL;
            //     2
            // }
            Self::Generic => {
                capabilities[0] = Capability::GenericPointer;
                1
            }
            Self::PushConstant => {
                capabilities[0] = Capability::Shader;
                1
            }
            Self::AtomicCounter => {
                capabilities[0] = Capability::AtomicStorage;
                1
            }
            Self::StorageBuffer => {
                capabilities[0] = Capability::Shader;
                1
            }
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Global<'a> {
    TypeVoid,
    TypeBool,
    TypeInt(TypeIntDetails),
    TypeFloat(TypeFloatDetails),
    TypeVector(TypeVectorDetails),
    TypeMatrix(TypeMatrixDetails),
    //TypeImage,
    TypeSampler,
    //TypeSampledImage,
    TypeArray(TypeArrayDetails<'a>),
    TypeRuntimeArray(TypeRuntimeArrayDetails<'a>),
    //TypeStruct(TypeStructDetails),
    //TypeOpaque(TypeOpaqueDetails),
    TypePointer(TypePointerDetails),
    TypeFunction(TypeFunctionDetails<'a>),
    //Other Types here in future
    ConstantTrue,
    ConstantFalse,
    Constant(ConstantDetails),
    Variable(VariableDetails),
}

#[derive(Clone, Copy, PartialEq)]
pub struct TypeIntDetails {
    pub bit_width: Word,
    pub is_signed: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub struct TypeFloatDetails {
    pub bit_width: Word,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ScalarType {
    Bool,
    Int(TypeIntDetails),
    Float(TypeFloatDetails),
}

#[derive(Clone, Copy, PartialEq)]
pub struct TypeVectorDetails {
    pub scalar_type: ScalarType,
    pub count: Word,
}

#[derive(Clone, Copy, PartialEq)]
pub struct TypeMatrixDetails {
    vector: TypeVectorDetails,
    count: Word,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Element<'a> {
    //Other Array "Element" Types Allowed / Disallowed...?
    Bool,
    Int(TypeIntDetails),
    Float(TypeFloatDetails),
    Vector(TypeVectorDetails),
    Matrix(TypeMatrixDetails),
    Array(TypeArrayDetails<'a>),
    //Realtime Array here...?
    //Struct(TypeStructDetails),
}

#[derive(Clone, Copy, PartialEq)]
pub struct TypeArrayDetails<'a> {
    element: &'a Element<'a>,
    length: u32,
}

#[derive(Clone, Copy, PartialEq)]
pub struct TypeRuntimeArrayDetails<'a> {
    element: &'a Element<'a>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Member<'a> {
    //Other Struct "Member" Types Allowed / Disallowed...?
    Bool,
    Int(TypeIntDetails),
    Float(TypeFloatDetails),
    Vector(TypeVectorDetails),
    Matrix(TypeMatrixDetails),
    Array(TypeArrayDetails<'a>),
    //Realtime Array here...?
    //Struct(TypeStructDetails),
}

// #[derive(Clone, Copy, PartialEq)]
// pub struct TypeStructDetails {
//     members: &'static [Member],
// }

#[derive(Clone, Copy, PartialEq)]
pub enum PointerType {
    //Other "Pointer" Types Allowed / Disallowed...?
    Bool,
    Int(TypeIntDetails),
    Float(TypeFloatDetails),
    Vector(TypeVectorDetails),
}

#[derive(Clone, Copy, PartialEq)]
pub struct TypePointerDetails {
    pub storage_class: StorageClass,
    pub pointer_type: PointerType,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ReturnType {
    //Other "Return" Types Allowed / Disallowed...?
    Void,
    Bool,
    Int(TypeIntDetails),
    Float(TypeFloatDetails),
    Vector(TypeVectorDetails),
}

impl<'a> ReturnType {
    pub fn get_global(&self) -> Global<'a> {
        match *self {
            ReturnType::Void => Global::TypeVoid,
            ReturnType::Bool => Global::TypeBool,
            ReturnType::Int(a) => Global::TypeInt(a),
            ReturnType::Float(a) => Global::TypeFloat(a),
            ReturnType::Vector(a) => Global::TypeVector(a),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ParameterType {
    //Other "Parameter" Types Allowed / Disallowed...?
    Bool,
    Int(TypeIntDetails),
    Float(TypeFloatDetails),
    Vector(TypeVectorDetails),
}

impl<'a> ParameterType {
    pub fn get_global(&self) -> Global<'a> {
        match *self {
            Self::Bool => Global::TypeBool,
            Self::Int(a) => Global::TypeInt(a),
            Self::Float(a) => Global::TypeFloat(a),
            Self::Vector(a) => Global::TypeVector(a),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct TypeFunctionDetails<'a> {
    pub return_type: ReturnType,
    pub parameter_types: &'a [ParameterType],
}

#[derive(Clone, Copy, PartialEq)]
pub enum ConstantDetails {
    //Int16(i16),
    Int32(i32),
    Int64(i64),
    //Unsigned16(u16),
    Unsigned32(u32),
    Unsigned64(u64),
    Float32(f32),
    Float64(f64),
}

#[derive(Clone, Copy, PartialEq)]
pub enum InitializerType {
    True,
    False,
    Constant(ConstantDetails),
}

#[derive(Clone, Copy, PartialEq)]
pub struct VariableDetails {
    pub pointer: TypePointerDetails, // Has the Storage Class
    pub initializer: Option<InitializerType>,
}

pub enum GlobalInstruction<'a> {
    Word(Word),
    Result,
    SubGlobal(Global<'a>),
}

impl<'a> Global<'a> {
    fn get_sub_global(&self, multi_stack: &mut Vec<Self>) -> Option<Self> {
        match self {
            Self::TypeVector(details) => match details.scalar_type {
                ScalarType::Bool => Some(Self::TypeBool),
                ScalarType::Int(a) => Some(Self::TypeInt(a)),
                ScalarType::Float(a) => Some(Self::TypeFloat(a)),
            },
            Self::TypeMatrix(details) => Some(Self::TypeVector(details.vector)),
            Self::TypeArray(details) => {
                multi_stack.push(match *details.element {
                    Element::Bool => Self::TypeBool,
                    Element::Int(a) => Self::TypeInt(a),
                    Element::Float(a) => Self::TypeFloat(a),
                    Element::Vector(a) => Self::TypeVector(a),
                    Element::Matrix(a) => Self::TypeMatrix(a),
                    Element::Array(a) => Self::TypeArray(a),
                    //Element::Struct(a) => Self::TypeStruct(a),
                });
                Some(Self::Constant(ConstantDetails::Unsigned32(details.length)))
            }
            Self::TypeRuntimeArray(details) => match *details.element {
                Element::Bool => Some(Self::TypeBool),
                Element::Int(a) => Some(Self::TypeInt(a)),
                Element::Float(a) => Some(Self::TypeFloat(a)),
                Element::Vector(a) => Some(Self::TypeVector(a)),
                Element::Matrix(a) => Some(Self::TypeMatrix(a)),
                Element::Array(a) => Some(Self::TypeArray(a)),
                //Element::Struct(a) => Self::TypeStruct(a),
            },
            Self::TypePointer(details) => match details.pointer_type {
                PointerType::Bool => Some(Self::TypeBool),
                PointerType::Int(a) => Some(Self::TypeInt(a)),
                PointerType::Float(a) => Some(Self::TypeFloat(a)),
                PointerType::Vector(a) => Some(Self::TypeVector(a)),
            },
            Self::TypeFunction(details) => {
                for d in details.parameter_types.iter().rev() {
                    multi_stack.push(d.get_global());
                }
                Some(details.return_type.get_global())
            }
            Self::ConstantTrue => Some(Self::TypeBool),
            Self::ConstantFalse => Some(Self::TypeBool),
            Self::Constant(details) => match details {
                ConstantDetails::Int32(_) => Some(Self::TypeInt(TypeIntDetails {
                    bit_width: 32,
                    is_signed: true,
                })),
                ConstantDetails::Int64(_) => Some(Self::TypeInt(TypeIntDetails {
                    bit_width: 64,
                    is_signed: true,
                })),
                ConstantDetails::Unsigned32(_) => Some(Self::TypeInt(TypeIntDetails {
                    bit_width: 32,
                    is_signed: false,
                })),
                ConstantDetails::Unsigned64(_) => Some(Self::TypeInt(TypeIntDetails {
                    bit_width: 64,
                    is_signed: false,
                })),
                ConstantDetails::Float32(_) => {
                    Some(Self::TypeFloat(TypeFloatDetails { bit_width: 32 }))
                }
                ConstantDetails::Float64(_) => {
                    Some(Self::TypeFloat(TypeFloatDetails { bit_width: 64 }))
                }
            },
            Self::Variable(details) => {
                if let Some(init) = details.initializer {
                    multi_stack.push(match init {
                        InitializerType::True => Self::ConstantTrue,
                        InitializerType::False => Self::ConstantFalse,
                        InitializerType::Constant(a) => Self::Constant(a),
                    });
                }
                Some(Self::TypePointer(details.pointer))
            }
            _ => None,
        }
    }

    pub fn push_sub_globals(&self, global_stack: &mut Vec<Self>) {
        let mut multi_stack = Vec::new();
        let mut sub_global = self.get_sub_global(&mut multi_stack);
        loop {
            while let Some(sg) = sub_global {
                global_stack.push(sg);
                sub_global = sg.get_sub_global(&mut multi_stack);
            }
            if let Some(ms) = multi_stack.pop() {
                sub_global = Some(ms);
            } else {
                break;
            }
        }
    }

    pub fn get_instructions(&self, global_instructions: &mut Vec<GlobalInstruction<'a>>) {
        match self {
            Self::TypeVoid => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::TypeVoid.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
            }
            Self::TypeBool => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::TypeBool.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
            }
            Self::TypeInt(details) => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::TypeInt.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
                global_instructions.push(GlobalInstruction::Word(details.bit_width));
                global_instructions.push(GlobalInstruction::Word(if details.is_signed {
                    1
                } else {
                    0
                }));
            }
            Self::TypeFloat(details) => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::TypeFloat.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
                global_instructions.push(GlobalInstruction::Word(details.bit_width));
            }
            Self::TypeVector(details) => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::TypeVector.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
                let scalar = match details.scalar_type {
                    ScalarType::Bool => Self::TypeBool,
                    ScalarType::Int(a) => Self::TypeInt(a),
                    ScalarType::Float(a) => Self::TypeFloat(a),
                };
                global_instructions.push(GlobalInstruction::SubGlobal(scalar));
                global_instructions.push(GlobalInstruction::Word(details.count));
            }
            Self::TypeMatrix(details) => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::TypeMatrix.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
                let vector = Self::TypeVector(details.vector);
                global_instructions.push(GlobalInstruction::SubGlobal(vector));
                global_instructions.push(GlobalInstruction::Word(details.count));
            }
            Self::TypeSampler => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::TypeSampler.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
            }
            Self::TypeArray(details) => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::TypeArray.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
                let element = match *details.element {
                    Element::Bool => Self::TypeBool,
                    Element::Int(a) => Self::TypeInt(a),
                    Element::Float(a) => Self::TypeFloat(a),
                    Element::Vector(a) => Self::TypeVector(a),
                    Element::Matrix(a) => Self::TypeMatrix(a),
                    Element::Array(a) => Self::TypeArray(a),
                    //Element::Struct(a) => Self::TypeStruct(a),
                };
                global_instructions.push(GlobalInstruction::SubGlobal(element));
                let length = Self::Constant(ConstantDetails::Unsigned32(details.length));
                global_instructions.push(GlobalInstruction::SubGlobal(length));
            }
            Self::TypeRuntimeArray(details) => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::TypeRuntimeArray.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
                let element = match *details.element {
                    Element::Bool => Self::TypeBool,
                    Element::Int(a) => Self::TypeInt(a),
                    Element::Float(a) => Self::TypeFloat(a),
                    Element::Vector(a) => Self::TypeVector(a),
                    Element::Matrix(a) => Self::TypeMatrix(a),
                    Element::Array(a) => Self::TypeArray(a),
                    //Element::Struct(a) => Self::TypeStruct(a),
                };
                global_instructions.push(GlobalInstruction::SubGlobal(element));
            }
            Self::TypePointer(details) => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::TypePointer.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
                global_instructions.push(GlobalInstruction::Word(details.storage_class.get_word()));
                let pointer_type = match details.pointer_type {
                    PointerType::Bool => Self::TypeBool,
                    PointerType::Int(a) => Self::TypeInt(a),
                    PointerType::Float(a) => Self::TypeFloat(a),
                    PointerType::Vector(a) => Self::TypeVector(a),
                };
                global_instructions.push(GlobalInstruction::SubGlobal(pointer_type));
            }
            Self::TypeFunction(details) => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::TypeFunction.get_word0(details.parameter_types.len() as u16),
                ));
                global_instructions.push(GlobalInstruction::Result);
                let return_type = details.return_type.get_global();
                global_instructions.push(GlobalInstruction::SubGlobal(return_type));
                for d in details.parameter_types.iter() {
                    global_instructions.push(GlobalInstruction::SubGlobal(d.get_global()));
                }
            }
            Self::ConstantTrue => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::ConstantTrue.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
                global_instructions.push(GlobalInstruction::SubGlobal(Self::TypeBool));
            }
            Self::ConstantFalse => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::ConstantFalse.get_fixed_word0(),
                ));
                global_instructions.push(GlobalInstruction::Result);
                global_instructions.push(GlobalInstruction::SubGlobal(Self::TypeBool));
            }
            Self::Constant(details) => match *details {
                ConstantDetails::Int32(v) => {
                    global_instructions
                        .push(GlobalInstruction::Word(OpcodeName::Constant.get_word0(0)));
                    global_instructions.push(GlobalInstruction::Result);
                    global_instructions.push(GlobalInstruction::SubGlobal(Self::TypeInt(
                        TypeIntDetails {
                            bit_width: 32,
                            is_signed: true,
                        },
                    )));
                    global_instructions.push(GlobalInstruction::Word(v as Word));
                }
                ConstantDetails::Int64(v) => {
                    global_instructions
                        .push(GlobalInstruction::Word(OpcodeName::Constant.get_word0(1)));
                    global_instructions.push(GlobalInstruction::Result);
                    global_instructions.push(GlobalInstruction::SubGlobal(Self::TypeInt(
                        TypeIntDetails {
                            bit_width: 64,
                            is_signed: true,
                        },
                    )));
                    let literal0 = ((v as u64) & 0xFFFFFFFF) as Word;
                    let literal1 = ((v as u64) >> 32) as Word;
                    global_instructions.push(GlobalInstruction::Word(literal0));
                    global_instructions.push(GlobalInstruction::Word(literal1));
                }
                ConstantDetails::Unsigned32(v) => {
                    global_instructions
                        .push(GlobalInstruction::Word(OpcodeName::Constant.get_word0(0)));
                    global_instructions.push(GlobalInstruction::Result);
                    global_instructions.push(GlobalInstruction::SubGlobal(Self::TypeInt(
                        TypeIntDetails {
                            bit_width: 32,
                            is_signed: false,
                        },
                    )));
                    global_instructions.push(GlobalInstruction::Word(v));
                }
                ConstantDetails::Unsigned64(v) => {
                    global_instructions
                        .push(GlobalInstruction::Word(OpcodeName::Constant.get_word0(1)));
                    global_instructions.push(GlobalInstruction::Result);
                    global_instructions.push(GlobalInstruction::SubGlobal(Self::TypeInt(
                        TypeIntDetails {
                            bit_width: 64,
                            is_signed: false,
                        },
                    )));
                    let literal0 = (v & 0xFFFFFFFF) as Word;
                    let literal1 = (v >> 32) as Word;
                    global_instructions.push(GlobalInstruction::Word(literal0));
                    global_instructions.push(GlobalInstruction::Word(literal1));
                }
                ConstantDetails::Float32(v) => {
                    global_instructions
                        .push(GlobalInstruction::Word(OpcodeName::Constant.get_word0(0)));
                    global_instructions.push(GlobalInstruction::Result);
                    global_instructions.push(GlobalInstruction::SubGlobal(Self::TypeFloat(
                        TypeFloatDetails { bit_width: 32 },
                    )));
                    global_instructions.push(GlobalInstruction::Word(v as Word));
                }
                ConstantDetails::Float64(v) => {
                    global_instructions
                        .push(GlobalInstruction::Word(OpcodeName::Constant.get_word0(1)));
                    global_instructions.push(GlobalInstruction::Result);
                    global_instructions.push(GlobalInstruction::SubGlobal(Self::TypeFloat(
                        TypeFloatDetails { bit_width: 64 },
                    )));
                    let float_convert = v as u64;
                    let literal0 = (float_convert & 0xFFFFFFFF) as Word;
                    let literal1 = (float_convert >> 32) as Word;
                    global_instructions.push(GlobalInstruction::Word(literal0));
                    global_instructions.push(GlobalInstruction::Word(literal1));
                }
            },
            Self::Variable(details) => {
                global_instructions.push(GlobalInstruction::Word(
                    OpcodeName::Variable.get_word0(if details.initializer.is_some() {
                        1
                    } else {
                        0
                    }),
                ));
                let pointer = Self::TypePointer(details.pointer);
                global_instructions.push(GlobalInstruction::SubGlobal(pointer));
                global_instructions.push(GlobalInstruction::Result);
                global_instructions.push(GlobalInstruction::Word(
                    details.pointer.storage_class.get_word(),
                ));
                if let Some(init) = details.initializer {
                    global_instructions.push(GlobalInstruction::SubGlobal(match init {
                        InitializerType::True => Self::ConstantTrue,
                        InitializerType::False => Self::ConstantFalse,
                        InitializerType::Constant(a) => Self::Constant(a),
                    }));
                }
            }
        }
    }
}

impl<'a> EnablingCapabilities for Global<'a> {
    fn get_required_capabilities(&self, capabilities: &mut [Capability]) -> usize {
        match self {
            Self::TypeMatrix(_) => {
                capabilities[0] = Capability::Matrix;
                1
            }
            Self::TypeRuntimeArray(_) => {
                capabilities[0] = Capability::Shader;
                1
            }
            //Self::Op
            _ => 0,
        }
    }
}

// pub struct TypeOpaque {
//     literal: Vec<Word>,
// }

// impl EnablingCapabilities for TypeOpaque {
//     fn get_required_capabilities(&self, capabilities: &mut [Capability]) -> usize {
//         capabilities[0] = Capability::Kernel;
//         1
//     }
// }

pub enum ExecutionModel {
    Vertex,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
    Fragment,
    GLCompute,
    Kernel,
    // TaskNV = 5_267,
    // MeshNV = 5_268,
    // RayGenerationNV = 5_313,
    // IntersectionNV = 5_314,
    // AnyHitNV = 5_315,
    // ClosestHitNV = 5_316,
    // MissNV = 5_317,
    // CallableNV = 5_318,
    // TaskEXT = 5_364,
    // MeshEXT = 5_365,
}
impl ExecutionModel {
    pub fn get_word(&self) -> Word {
        match self {
            Self::Vertex => 0,
            Self::TessellationControl => 1,
            Self::TessellationEvaluation => 2,
            Self::Geometry => 3,
            Self::Fragment => 4,
            Self::GLCompute => 5,
            Self::Kernel => 6,
        }
    }
}

impl EnablingCapabilities for ExecutionModel {
    fn get_required_capabilities(&self, capabilities: &mut [Capability]) -> usize {
        match self {
            Self::Vertex => {
                capabilities[0] = Capability::Shader;
                1
            }
            Self::TessellationControl => {
                capabilities[0] = Capability::Tessellation;
                1
            }
            Self::TessellationEvaluation => {
                capabilities[0] = Capability::Tessellation;
                1
            }
            Self::Geometry => {
                capabilities[0] = Capability::Geometry;
                1
            }
            Self::Fragment => {
                capabilities[0] = Capability::Shader;
                1
            }
            Self::GLCompute => {
                capabilities[0] = Capability::Shader;
                1
            }
            Self::Kernel => {
                capabilities[0] = Capability::Kernel;
                1
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum GeometryExecutionMode {
    Invocations(Word),
    InputPoints,
    InputLines,
    InputLinesAdjacency,
    Triangles,
    InputTrianglesAdjacency,
    //OutputVertices(Word),
    //OutputPoints,
    OutputLineStrip,
    OutputTriangleStrip,
}

#[derive(Clone, Copy)]
pub enum TessellationExecutionMode {
    SpacingEqual,
    SpacingFractionalEven,
    SpacingFractionalOdd,
    VertexOrderCw,
    VertexOrderCcw,
    PointMode,
    Triangles,
    Quads,
    Isolines,
    //OutputVertices(Word),
}

#[derive(Clone, Copy)]
pub enum FragmentExecutionMode {
    PixelCenterInteger,
    OriginUpperLeft,
    OriginLowerLeft,
    EarlyFragmentTests,
    DepthReplacing,
    DepthGreater,
    DepthLess,
    DepthUnchanged,
}

#[derive(Clone, Copy)]
pub enum GLComputeExecutionMode {
    LocalSize((Word, Word, Word)),
}

#[derive(Clone, Copy)]
pub enum KernelExecutionMode {
    LocalSize((Word, Word, Word)),
    LocalSizeHint((Word, Word, Word)),
    VecTypeHint(Word),
    ContractionOff,
}

pub enum ExecutionMode {
    Geometry(GeometryExecutionMode),
    Tessellation(TessellationExecutionMode),
    Fragment(FragmentExecutionMode),
    GLCompute(GLComputeExecutionMode),
    Kernel(KernelExecutionMode),
    //Xfb,
    // Initializer,
    // Finalizer,
    // SubgroupSize(Word),
    // SubgroupsPerWorkgroup(Word),
    // SubgroupsPerWorkgroupId = 37,
    // LocalSizeId = 38,
    // LocalSizeHintId = 39,
    // NonCoherentColorAttachmentReadEXT = 4_169,
    // NonCoherentDepthAttachmentReadEXT = 4_170,
    // NonCoherentStencilAttachmentReadEXT = 4_171,
    // SubgroupUniformControlFlowKHR = 4_421,
    // PostDepthCoverage = 4_446,
    // DenormPreserve = 4_459,
    // DenormFlushToZero = 4_460,
    // SignedZeroInfNanPreserve = 4_461,
    // RoundingModeRTE = 4_462,
    // RoundingModeRTZ = 4_463,
    // EarlyAndLateFragmentTestsAMD = 5_017,
    // StencilRefReplacingEXT = 5_027,
    // CoalescingAMDX = 5_069,
    // MaxNodeRecursionAMDX = 5_071,
    // StaticNumWorkgroupsAMDX = 5_072,
    // ShaderIndexAMDX = 5_073,
    // MaxNumWorkgroupsAMDX = 5_077,
    // StencilRefUnchangedFrontAMD = 5_079,
    // StencilRefGreaterFrontAMD = 5_080,
    // StencilRefLessFrontAMD = 5_081,
    // StencilRefUnchangedBackAMD = 5_082,
    // StencilRefGreaterBackAMD = 5_083,
    // StencilRefLessBackAMD = 5_084,
    // OutputLinesNV = 5_269,
    // OutputPrimitivesNV = 5_270,
    // DerivativeGroupQuadsNV = 5_289,
    // DerivativeGroupLinearNV = 5_290,
    // OutputTrianglesNV = 5_298,
    // PixelInterlockOrderedEXT = 5_366,
    // PixelInterlockUnorderedEXT = 5_367,
    // SampleInterlockOrderedEXT = 5_368,
    // SampleInterlockUnorderedEXT = 5_369,
    // ShadingRateInterlockOrderedEXT = 5_370,
    // ShadingRateInterlockUnorderedEXT = 5_371,
    // SharedLocalMemorySizeINTEL = 5_618,
    // RoundingModeRTPINTEL = 5_620,
    // RoundingModeRTNINTEL = 5_621,
    // FloatingPointModeALTINTEL = 5_622,
    // FloatingPointModeIEEEINTEL = 5_623,
    // MaxWorkgroupSizeINTEL = 5_893,
    // MaxWorkDimINTEL = 5_894,
    // NoGlobalOffsetINTEL = 5_895,
    // NumSIMDWorkitemsINTEL = 5_896,
    // SchedulerTargetFmaxMhzINTEL = 5_903,
    // StreamingInterfaceINTEL = 6_154,
    // RegisterMapInterfaceINTEL = 6_160,
    // NamedBarrierCountINTEL = 6_417,
}
impl ExecutionMode {
    pub fn get_word_and_literals(&self, literals: &mut [Word]) -> (Word, u16) {
        let word = match self {
            Self::Geometry(g) => match g {
                GeometryExecutionMode::Invocations(a) => {
                    literals[0] = *a;
                    return (0, 1);
                }
                GeometryExecutionMode::InputPoints => 19,
                GeometryExecutionMode::InputLines => 20,
                GeometryExecutionMode::InputLinesAdjacency => 21,
                GeometryExecutionMode::Triangles => 22,
                GeometryExecutionMode::InputTrianglesAdjacency => 23,
                GeometryExecutionMode::OutputLineStrip => 28,
                GeometryExecutionMode::OutputTriangleStrip => 29,
            },
            Self::Tessellation(t) => match t {
                TessellationExecutionMode::SpacingEqual => 1,
                TessellationExecutionMode::SpacingFractionalEven => 2,
                TessellationExecutionMode::SpacingFractionalOdd => 3,
                TessellationExecutionMode::VertexOrderCw => 4,
                TessellationExecutionMode::VertexOrderCcw => 5,
                TessellationExecutionMode::PointMode => 10,
                TessellationExecutionMode::Triangles => 22,
                TessellationExecutionMode::Quads => 24,
                TessellationExecutionMode::Isolines => 25,
            },
            Self::Fragment(f) => match f {
                FragmentExecutionMode::PixelCenterInteger => 6,
                FragmentExecutionMode::OriginUpperLeft => 7,
                FragmentExecutionMode::OriginLowerLeft => 8,
                FragmentExecutionMode::EarlyFragmentTests => 9,
                FragmentExecutionMode::DepthReplacing => 12,
                FragmentExecutionMode::DepthGreater => 14,
                FragmentExecutionMode::DepthLess => 15,
                FragmentExecutionMode::DepthUnchanged => 16,
            },
            Self::GLCompute(gl) => match gl {
                GLComputeExecutionMode::LocalSize((a, b, c)) => {
                    literals[0] = *a;
                    literals[1] = *b;
                    literals[2] = *c;
                    return (17, 3);
                }
            },
            Self::Kernel(k) => match k {
                KernelExecutionMode::LocalSize((a, b, c)) => {
                    literals[0] = *a;
                    literals[1] = *b;
                    literals[2] = *c;
                    return (17, 3);
                }
                KernelExecutionMode::LocalSizeHint((a, b, c)) => {
                    literals[0] = *a;
                    literals[1] = *b;
                    literals[2] = *c;
                    return (18, 3);
                }
                KernelExecutionMode::VecTypeHint(a) => {
                    literals[0] = *a;
                    return (30, 1);
                }
                KernelExecutionMode::ContractionOff => 31,
            },
        };

        // let word = match self {
        //     Self::Xfb => 11,
        //     Self::OutputVertices => {
        //         26
        //     }
        //     Self::OutputPoints => {
        //         27
        //     }
        //     Self::Initializer => 33,
        //     Self::Finalizer => 34,
        //     Self::SubgroupSize(a) => {
        //         literals[0] = *a;
        //         return (35, 1);
        //     }
        //     Self::SubgroupsPerWorkgroup(a) => {
        //         literals[0] = *a;
        //         return (36, 1);
        //     } // Self::SubgroupsPerWorkgroupId => {
        //           capabilities[0] = Capability::SubgroupDispatch;
        //           1
        //       }
        //       Self::LocalSizeId => {
        //           capabilities[0] = Capability::Kernel;
        //           1
        //       }
        //       Self::LocalSizeHintId => {
        //           capabilities[0] = Capability::Kernel;
        //           1
        //       }
        // };
        (word, 0)
    }
}

impl EnablingCapabilities for ExecutionMode {
    fn get_required_capabilities(&self, capabilities: &mut [Capability]) -> usize {
        // match self {
        //     Self::Invocations(_) => {
        //         capabilities[0] = Capability::Geometry;
        //         1
        //     }
        //     Self::SpacingEqual => {
        //         capabilities[0] = Capability::Tessellation;
        //         1
        //     }
        //     Self::SpacingFractionalEven => {
        //         capabilities[0] = Capability::Tessellation;
        //         1
        //     }
        //     Self::SpacingFractionalOdd => {
        //         capabilities[0] = Capability::Tessellation;
        //         1
        //     }
        //     Self::VertexOrderCw => {
        //         capabilities[0] = Capability::Tessellation;
        //         1
        //     }
        //     Self::VertexOrderCcw => {
        //         capabilities[0] = Capability::Tessellation;
        //         1
        //     }
        //     Self::PixelCenterInteger => {
        //         capabilities[0] = Capability::Shader;
        //         1
        //     }
        //     Self::OriginUpperLeft => {
        //         capabilities[0] = Capability::Shader;
        //         1
        //     }
        //     Self::OriginLowerLeft => {
        //         capabilities[0] = Capability::Shader;
        //         1
        //     }
        //     Self::EarlyFragmentTests => {
        //         capabilities[0] = Capability::Shader;
        //         1
        //     }
        //     Self::PointMode => {
        //         capabilities[0] = Capability::Tessellation;
        //         1
        //     }
        //     Self::Xfb => {
        //         capabilities[0] = Capability::TransformFeedback;
        //         1
        //     }
        //     Self::DepthReplacing => {
        //         capabilities[0] = Capability::Shader;
        //         1
        //     }
        //     Self::DepthGreater => {
        //         capabilities[0] = Capability::Shader;
        //         1
        //     }
        //     Self::DepthLess => {
        //         capabilities[0] = Capability::Shader;
        //         1
        //     }
        //     Self::DepthUnchanged => {
        //         capabilities[0] = Capability::Shader;
        //         1
        //     }
        //     // Self::LocalSize => {
        //     //     capabilities[0] = Capability::Kernel;
        //     //     1
        //     // }
        //     Self::LocalSizeHint(_) => {
        //         capabilities[0] = Capability::Kernel;
        //         1
        //     }
        //     Self::InputPoints => {
        //         capabilities[0] = Capability::Geometry;
        //         1
        //     }
        //     Self::InputLines => {
        //         capabilities[0] = Capability::Geometry;
        //         1
        //     }
        //     Self::InputLinesAdjacency => {
        //         capabilities[0] = Capability::Geometry;
        //         1
        //     }
        //     Self::Triangles => {
        //         capabilities[0] = Capability::Geometry;
        //         capabilities[1] = Capability::Tessellation;
        //         2
        //     }
        //     Self::InputTrianglesAdjacency => {
        //         capabilities[0] = Capability::Geometry;
        //         1
        //     }
        //     Self::Quads => {
        //         capabilities[0] = Capability::Tessellation;
        //         1
        //     }
        //     Self::Isolines => {
        //         capabilities[0] = Capability::Tessellation;
        //         1
        //     }
        //     // Self::OutputVertices => {
        //     //     capabilities[0] = Capability::Geometry;
        //     //     capabilities[1] = Capability::Tessellation;
        //     //     capabilities[2] = Capability::MeshShadingNV;
        //     //     capabilities[3] = Capability::MeshShadingEXT;
        //     //     4
        //     // }
        //     // Self::OutputPoints => {
        //     //     capabilities[0] = Capability::Geometry;
        //     //     capabilities[1] = Capability::MeshShadingNV;
        //     //     capabilities[2] = Capability::MeshShadingEXT;
        //     //     3
        //     // }
        //     Self::OutputLineStrip => {
        //         capabilities[0] = Capability::Geometry;
        //         1
        //     }
        //     Self::OutputTriangleStrip => {
        //         capabilities[0] = Capability::Geometry;
        //         1
        //     }
        //     Self::VecTypeHint(_) => {
        //         capabilities[0] = Capability::Kernel;
        //         1
        //     }
        //     Self::ContractionOff => {
        //         capabilities[0] = Capability::Kernel;
        //         1
        //     }
        //     Self::Initializer => {
        //         capabilities[0] = Capability::Kernel;
        //         1
        //     }
        //     Self::Finalizer => {
        //         capabilities[0] = Capability::Kernel;
        //         1
        //     }
        //     Self::SubgroupSize(_) => {
        //         capabilities[0] = Capability::SubgroupDispatch;
        //         1
        //     }
        //     Self::SubgroupsPerWorkgroup(_) => {
        //         capabilities[0] = Capability::SubgroupDispatch;
        //         1
        //     }
        //     // Self::SubgroupsPerWorkgroupId => {
        //     //     capabilities[0] = Capability::SubgroupDispatch;
        //     //     1
        //     // }
        //     // // Self::LocalSizeId => {
        //     // //     capabilities[0] = Capability::Kernel;
        //     // //     1
        //     // // }
        //     // Self::LocalSizeHintId => {
        //     //     capabilities[0] = Capability::Kernel;
        //     //     1
        //     // }
        //     _ => 0,
        // }
        match self {
            Self::Geometry(g) => match g {
                GeometryExecutionMode::Triangles => {
                    capabilities[0] = Capability::Geometry;
                    capabilities[1] = Capability::Tessellation;
                    2
                }
                _ => {
                    capabilities[0] = Capability::Geometry;
                    1
                }
            },
            Self::Tessellation(t) => match t {
                TessellationExecutionMode::Triangles => {
                    capabilities[0] = Capability::Tessellation;
                    capabilities[1] = Capability::Geometry;
                    2
                }
                _ => {
                    capabilities[0] = Capability::Tessellation;
                    1
                }
            },
            Self::Fragment(_f) => {
                capabilities[0] = Capability::Shader;
                1
            }
            Self::GLCompute(gl) => match gl {
                GLComputeExecutionMode::LocalSize(_) => {
                    capabilities[0] = Capability::Shader;
                    capabilities[1] = Capability::Kernel;
                    2
                }
            },
            Self::Kernel(k) => match k {
                KernelExecutionMode::LocalSize(_) => {
                    capabilities[0] = Capability::Kernel;
                    capabilities[1] = Capability::Shader;
                    2
                }
                _ => {
                    capabilities[0] = Capability::Kernel;
                    1
                }
            },
        }
    }
}

#[derive(Clone, Copy)]
pub enum VertexBuiltIn {
    Position,
}

#[derive(Clone, Copy)]
pub enum GeometryBuiltIn {
    Position(bool),
}

#[derive(Clone, Copy)]
pub enum TessellationBuiltIn {
    Position(bool),
}

#[derive(Clone, Copy)]
pub enum FragmentBuiltIn {
    FragCoord,
    PointCoord,
    VertexIndex,
    InstanceIndex,
}

pub enum BuiltIn {
    Vertex(VertexBuiltIn),
    Geometry(GeometryBuiltIn),
    Tessellation(TessellationBuiltIn),
    Fragment(FragmentBuiltIn),
}
impl<'a> BuiltIn {
    pub fn get_pointer_type(&self) -> PointerType {
        match self {
            Self::Vertex(v) => match v {
                VertexBuiltIn::Position => PointerType::Vector(TypeVectorDetails {
                    scalar_type: ScalarType::Float(TypeFloatDetails { bit_width: 32 }),
                    count: 4,
                }),
            },
            Self::Geometry(g) => match g {
                GeometryBuiltIn::Position(_) => PointerType::Vector(TypeVectorDetails {
                    scalar_type: ScalarType::Float(TypeFloatDetails { bit_width: 32 }),
                    count: 4,
                }),
            },
            Self::Tessellation(t) => match t {
                TessellationBuiltIn::Position(_) => PointerType::Vector(TypeVectorDetails {
                    scalar_type: ScalarType::Float(TypeFloatDetails { bit_width: 32 }),
                    count: 4,
                }),
            },
            Self::Fragment(f) => match f {
                FragmentBuiltIn::FragCoord => PointerType::Vector(TypeVectorDetails {
                    scalar_type: ScalarType::Float(TypeFloatDetails { bit_width: 32 }),
                    count: 4,
                }),
                FragmentBuiltIn::PointCoord => PointerType::Vector(TypeVectorDetails {
                    scalar_type: ScalarType::Float(TypeFloatDetails { bit_width: 32 }),
                    count: 2,
                }),
                FragmentBuiltIn::VertexIndex => PointerType::Int(TypeIntDetails {
                    bit_width: 32,
                    is_signed: false,
                }),
                FragmentBuiltIn::InstanceIndex => PointerType::Int(TypeIntDetails {
                    bit_width: 32,
                    is_signed: false,
                }),
            },
        }
    }

    pub fn get_storage_class(&self) -> StorageClass {
        match self {
            Self::Vertex(v) => match v {
                VertexBuiltIn::Position => StorageClass::Output,
            },
            Self::Geometry(g) => match *g {
                GeometryBuiltIn::Position(is_input) => {
                    if is_input {
                        StorageClass::Input
                    } else {
                        StorageClass::Output
                    }
                }
            },
            Self::Tessellation(t) => match *t {
                TessellationBuiltIn::Position(is_input) => {
                    if is_input {
                        StorageClass::Input
                    } else {
                        StorageClass::Output
                    }
                }
            },
            Self::Fragment(f) => match f {
                FragmentBuiltIn::FragCoord => StorageClass::Input,
                FragmentBuiltIn::PointCoord => StorageClass::Input,
                FragmentBuiltIn::VertexIndex => StorageClass::Input,
                FragmentBuiltIn::InstanceIndex => StorageClass::Input,
            },
        }
    }

    pub fn get_global_and_is_output(&self) -> (Global<'a>, bool) {
        let pointer_type = self.get_pointer_type();
        let storage_class = self.get_storage_class();
        match storage_class {
            StorageClass::Input => (
                Global::Variable(VariableDetails {
                    pointer: TypePointerDetails {
                        storage_class: StorageClass::Input,
                        pointer_type,
                    },
                    initializer: None,
                }),
                false,
            ),
            StorageClass::Output => (
                Global::Variable(VariableDetails {
                    pointer: TypePointerDetails {
                        storage_class: StorageClass::Output,
                        pointer_type,
                    },
                    initializer: None,
                }),
                true,
            ),
            _ => panic!("Invalid Builtin Storage Class!"),
        }
    }

    pub fn get_word(&self) -> Word {
        match self {
            Self::Vertex(v) => match v {
                VertexBuiltIn::Position => 0,
            },
            Self::Geometry(g) => match g {
                GeometryBuiltIn::Position(_) => 0,
            },
            Self::Tessellation(t) => match t {
                TessellationBuiltIn::Position(_) => 0,
            },
            Self::Fragment(f) => match f {
                FragmentBuiltIn::FragCoord => 15,
                FragmentBuiltIn::PointCoord => 16,
                FragmentBuiltIn::VertexIndex => 42,
                FragmentBuiltIn::InstanceIndex => 43,
            },
        }
    }
}

pub enum Decoration {
    RelaxedPrecision,
    SpecId(Word),
    Block,
    BufferBlock,
    RowMajor,
    ColMajor,
    ArrayStride(Word),
    MatrixStride(Word),
    GLSLShared,
    GLSLPacked,
    CPacked,
    BuiltIn(BuiltIn),
    NoPerspective,
    Flat,
    Patch,
    Centroid,
    Sample,
    Invariant,
    Restrict,
    Aliased,
    Volatile,
    Constant,
    Coherent,
    NonWritable,
    NonReadable,
    Uniform,
    UniformId(Word),
    SaturatedConversion,
    Stream(Word),
    Location(Word),
    Component(Word),
    Index(Word),
    Binding(Word),
    DescriptorSet(Word),
    Offset(Word),
    XfbBuffer(Word),
    XfbStride(Word),
    // FuncParamAttr(FunctionParameterAttribute),
    // FPRoundingMode(FPRoundingMode),
    // FPFastMathMode(FPFastMathMode),
    // LinkageAttributes(String, LinkageType),
    NoContraction,
    InputAttachmentIndex(Word),
    Alignment(Word),
    MaxByteOffset(Word),
    // AlignmentId(Word),
    // MaxByteOffsetId(Word),
    // NoSignedWrap,
    // NoUnsignedWrap,
    // WeightTextureQCOM,
    // BlockMatchTextureQCOM,
    // ExplicitInterpAMD,
    // NodeSharesPayloadLimitsWithAMDX(Word),
    // NodeMaxPayloadsAMDX(Word),
    // TrackFinishWritingAMDX,
    // PayloadNodeNameAMDX(String),
    // OverrideCoverageNV,
    // PassthroughNV,
    // ViewportRelativeNV,
    // SecondaryViewportRelativeNV(u32),
    // PerPrimitiveNV,
    // PerPrimitiveEXT,
    // PerViewNV,
    // PerTaskNV,
    // PerVertexKHR,
    // PerVertexNV,
    // NonUniform,
    // NonUniformEXT,
    // RestrictPointer,
    // RestrictPointerEXT,
    // AliasedPointer,
    // AliasedPointerEXT,
    // HitObjectShaderRecordBufferNV,
    // BindlessSamplerNV,
    // BindlessImageNV,
    // BoundSamplerNV,
    // BoundImageNV,
    // SIMTCallINTEL(u32),
    // ReferencedIndirectlyINTEL,
    // ClobberINTEL(String),
    // SideEffectsINTEL,
    // VectorComputeVariableINTEL,
    // FuncParamIOKindINTEL(u32),
    // VectorComputeFunctionINTEL,
    // StackCallINTEL,
    // GlobalVariableOffsetINTEL(u32),
    // CounterBuffer(Word),
    // HlslCounterBufferGOOGLE(Word),
    // UserSemantic(String),
    // HlslSemanticGOOGLE(String),
    // UserTypeGOOGLE(String),
    // FunctionRoundingModeINTEL(u32, FPRoundingMode),
    // FunctionDenormModeINTEL(u32, FPDenormMode),
    // RegisterINTEL,
    // MemoryINTEL(String),
    // NumbanksINTEL(u32),
    // BankwidthINTEL(u32),
    // MaxPrivateCopiesINTEL(u32),
    // SinglepumpINTEL,
    // DoublepumpINTEL,
    // MaxReplicatesINTEL(u32),
    // SimpleDualPortINTEL,
    // MergeINTEL(String, String),
    // BankBitsINTEL(Vec<u32>),
    // ForcePow2DepthINTEL(u32),
    // BurstCoalesceINTEL,
    // CacheSizeINTEL(u32),
    // DontStaticallyCoalesceINTEL,
    // PrefetchINTEL(u32),
    // StallEnableINTEL,
    // FuseLoopsInFunctionINTEL,
    // MathOpDSPModeINTEL(u32, u32),
    // AliasScopeINTEL(Word),
    // NoAliasINTEL(Word),
    // InitiationIntervalINTEL(u32),
    // MaxConcurrencyINTEL(u32),
    // PipelineEnableINTEL(u32),
    // BufferLocationINTEL(u32),
    // IOPipeStorageINTEL(u32),
    // FunctionFloatingPointModeINTEL(u32, FPOperationMode),
    // SingleElementVectorINTEL,
    // VectorComputeCallableFunctionINTEL,
    // MediaBlockIOINTEL,
    // InitModeINTEL(InitializationModeQualifier),
    // ImplementInRegisterMapINTEL(u32),
    // HostAccessINTEL(HostAccessQualifier, String),
    // FPMaxErrorDecorationINTEL(u32),
    // LatencyControlLabelINTEL(u32),
    // LatencyControlConstraintINTEL(u32, u32, u32),
    // ConduitKernelArgumentINTEL,
    // RegisterMapKernelArgumentINTEL,
    // MMHostInterfaceAddressWidthINTEL(u32),
    // MMHostInterfaceDataWidthINTEL(u32),
    // MMHostInterfaceLatencyINTEL(u32),
    // MMHostInterfaceReadWriteModeINTEL(AccessQualifier),
    // MMHostInterfaceMaxBurstINTEL(u32),
    // MMHostInterfaceWaitRequestINTEL(u32),
    // StableKernelArgumentINTEL,
    // CacheControlLoadINTEL(u32, LoadCacheControl),
    // CacheControlStoreINTEL(u32, StoreCacheControl),
}
impl Decoration {
    pub fn get_word(&self) -> Word {
        match self {
            Self::BuiltIn(_b) => 11,
            Self::Location(_) => 30,
            Self::DescriptorSet(_) => 34,
            Self::Binding(_) => 33,
            Self::Flat => 14,
            _ => panic!("Decoration Not Currently Supported!"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum LocalVariableType {
    //Other "Local Variable" Types Allowed / Disallowed...?
    Bool,
    Int(TypeIntDetails),
    Float(TypeFloatDetails),
    Vector(TypeVectorDetails),
}
impl<'a> LocalVariableType {
    pub fn get_global(&self) -> Global<'a> {
        match *self {
            Self::Bool => Global::TypeBool,
            Self::Int(a) => Global::TypeInt(a),
            Self::Float(a) => Global::TypeFloat(a),
            Self::Vector(a) => Global::TypeVector(a),
        }
    }
}
