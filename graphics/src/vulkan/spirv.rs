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

pub mod def;
use def::{
    Capability, Decoration, EnablingCapabilities, ExecutionModel, Global, GlobalInstruction,
    LocalVariableType, OpcodeName, StorageClass, TypeFunctionDetails, VariableDetails, Word,
};

#[derive(Debug)]
pub enum Error {
    InvalidGlobal,
    UnsupportedExecutionModel,
    InvalidName,
    InvalidCapability,
}

struct Capabilities {
    capabilities: Vec<Capability>,
}

impl Capabilities {
    fn new(initial_capability: Capability) -> Self {
        Self {
            capabilities: vec![initial_capability],
        }
    }

    fn check_capability(&self, capability: &Capability) -> bool {
        for c in &self.capabilities {
            if c != capability {
                let mut implicit_c = c.get_implicit();
                while let Some(ci) = implicit_c {
                    if &ci != capability {
                        implicit_c = ci.get_implicit();
                    } else {
                        return true;
                    }
                }
            } else {
                return true;
            }
        }
        false
    }

    fn add_capability(&mut self, capability: Capability) {
        if !self.check_capability(&capability) {
            // Better add in future that takes the implicits into account
            self.capabilities.push(capability);
        }
    }

    fn add_words(&self, word_stream: &mut Vec<Word>) {
        let word0 = OpcodeName::Capability.get_fixed_word0();
        for capability in &self.capabilities {
            word_stream.push(word0);
            word_stream.push(*capability as Word);
        }
    }
}

struct Ids {
    next_id: Word,
}

impl Ids {
    fn new() -> Self {
        Self { next_id: 1 }
    }

    fn get_next_id(&mut self) -> Word {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn get_id_bound(&self) -> Word {
        self.next_id
    }
}

struct MemoryModel {
    addressing_model: def::AddressingModel,
    memory_model: def::MemoryModel,
}

impl MemoryModel {
    fn add_words(&self, word_stream: &mut Vec<Word>) {
        word_stream.push(OpcodeName::MemoryModel.get_fixed_word0());
        word_stream.push(self.addressing_model.get_word());
        word_stream.push(self.memory_model.get_word());
    }
}

const MAX_UTF8_NAME_BYTE_LENGTH: usize = 31;

struct EntryPoint<'a> {
    execution_model: ExecutionModel,
    function_id: Word,
    name: [u8; MAX_UTF8_NAME_BYTE_LENGTH],
    name_len: usize,
    interface: Vec<Global<'a>>,
    execution_modes: Vec<def::ExecutionMode>,
}

impl<'a> EntryPoint<'a> {
    fn new(
        execution_model: ExecutionModel,
        function_id: Word,
        name_str: &str,
        interface: Vec<Global<'a>>,
        execution_modes: Vec<def::ExecutionMode>,
    ) -> Result<Self, Error> {
        let mut name = [0; MAX_UTF8_NAME_BYTE_LENGTH];
        let mut name_len = 0;

        for c in name_str.chars() {
            let new_name_len = name_len + c.len_utf8();
            if new_name_len > MAX_UTF8_NAME_BYTE_LENGTH {
                break;
            }
            let name_subslice = &mut name[name_len..new_name_len];
            c.encode_utf8(name_subslice);
            name_len = new_name_len;
        }

        if name_len == 0 {
            return Err(Error::InvalidName);
        }

        Ok(Self {
            execution_model,
            function_id,
            name,
            name_len,
            interface,
            execution_modes,
        })
    }

    fn add_words(&self, word_stream: &mut Vec<Word>, globals: &GlobalDeclarations) {
        let name_word_count = (self.name_len + 4) >> 2;
        let additional_arguments = (name_word_count + self.interface.len() - 1) as u16;
        let word0 = OpcodeName::EntryPoint.get_word0(additional_arguments);
        word_stream.push(word0);
        word_stream.push(self.execution_model.get_word());
        word_stream.push(self.function_id);
        def::add_str_bytes(&self.name[0..self.name_len], word_stream);
        for g in &self.interface {
            let id = globals.check_global_id(g).unwrap();
            word_stream.push(id);
        }
    }

    fn add_execution_mode_words(&self, word_stream: &mut Vec<Word>) {
        let mut literals = [0; 8];
        for em in &self.execution_modes {
            let (mode_word, additional_arguments) = em.get_word_and_literals(&mut literals);
            let word0 = OpcodeName::ExecutionMode.get_word0(additional_arguments);
            word_stream.push(word0);
            word_stream.push(self.function_id);
            word_stream.push(mode_word);
            for l in literals.iter().take(additional_arguments as usize) {
                word_stream.push(*l);
            }
        }
    }
}

struct GlobalDeclarations<'a> {
    globals: Vec<(Global<'a>, Word)>,
}

impl<'a> GlobalDeclarations<'a> {
    fn new() -> Self {
        Self {
            globals: Vec::new(),
        }
    }

    fn get_global_id(&mut self, ids: &mut Ids, global: Global<'a>) -> Word {
        for (g, g_id) in &self.globals {
            if *g == global {
                return *g_id;
            }
        }
        let mut global_stack = Vec::new();
        global.push_sub_globals(&mut global_stack);
        while let Some(global_test) = global_stack.pop() {
            let mut found = false;
            for (g, _g_id) in &self.globals {
                if *g == global_test {
                    found = true;
                    break;
                }
            }
            if !found {
                let id = ids.get_next_id();
                self.globals.push((global_test, id));
            }
        }

        let id = ids.get_next_id();
        self.globals.push((global, id));

        id
    }

    fn check_global_id(&self, global: &Global<'a>) -> Option<Word> {
        for (g, g_id) in &self.globals {
            if *g == *global {
                return Some(*g_id);
            }
        }
        None
    }

    fn add_words(&self, word_stream: &mut Vec<Word>) {
        let mut global_instructions = Vec::new();
        for (g, g_id) in &self.globals {
            global_instructions.clear();
            g.get_instructions(&mut global_instructions);
            for gi in &global_instructions {
                match gi {
                    GlobalInstruction::Word(v) => {
                        word_stream.push(*v);
                    }
                    GlobalInstruction::Result => {
                        word_stream.push(*g_id);
                    }
                    GlobalInstruction::SubGlobal(a) => {
                        for (g, g_id) in &self.globals {
                            if *g == *a {
                                word_stream.push(*g_id);
                                break;
                            }
                        }
                        // No error checking here yet!
                    }
                }
            }
        }
    }
}

//#[derive(Clone, Copy, PartialEq)]
pub enum Instruction {
    Label(Word),
    Return,
    ReturnValue(Word),
    Branch(Word),
    Store,
    Load,
}
impl Instruction {
    pub fn add_words(&self, _word_stream: &mut [Word]) {}
}

struct Function<'a> {
    function_type: Global<'a>,
    id: Word,
    control: def::FunctionControl,
    parameters: Vec<(Global<'a>, Word)>,
    label0_id: Word,
    local_vars: Vec<(LocalVariableType, Word)>,
    instructions: Vec<Instruction>,
}

impl<'a> Function<'a> {
    fn add_words(&self, word_stream: &mut Vec<Word>, globals: &GlobalDeclarations) {
        let word0 = OpcodeName::Function.get_fixed_word0();
        word_stream.push(word0);

        match self.function_type {
            Global::TypeFunction(details) => {
                let type_id = globals
                    .check_global_id(&details.return_type.get_global())
                    .unwrap();
                word_stream.push(type_id);
            }
            _ => panic!("Incorrect Global Function Type!"),
        }

        word_stream.push(self.id);
        word_stream.push(self.control.get_word());
        let type_id = globals.check_global_id(&self.function_type).unwrap();
        word_stream.push(type_id);

        let word0 = OpcodeName::FunctionParameter.get_fixed_word0();
        for (g, id) in &self.parameters {
            word_stream.push(word0);
            let type_id = globals.check_global_id(g).unwrap();
            word_stream.push(type_id);
            word_stream.push(*id);
        }

        let word0 = OpcodeName::Label.get_fixed_word0();
        word_stream.push(word0);
        word_stream.push(self.label0_id);

        let _word0 = OpcodeName::Variable.get_fixed_word0();
        // for (g, id) in &self.local_vars {

        // }

        // for i in &self.instructions {
        //     i.push
        // }

        let word0 = OpcodeName::FunctionEnd.get_fixed_word0();
        word_stream.push(word0);
    }
}

pub struct Module<'a> {
    capabilities: Capabilities,
    ids: Ids,
    //extensions: ,
    glsl_builtin_functions: Option<Word>,
    memory_model: MemoryModel,
    entry_points: Vec<EntryPoint<'a>>,
    //debug_a: ,
    //debug_b: ,
    //debug_c: ,
    //annotations: ,
    decorations: Vec<(Decoration, Global<'a>)>,
    global_declarations: GlobalDeclarations<'a>,
    //type_declarations: Vec<u32>,
    functions: Vec<Function<'a>>,
}

const MAIN_FUNCTION_PARAMETER_TYPES: [def::ParameterType; 0] = [def::ParameterType::Bool; 0];

enum IdType {
    Function,
    GlobalInput,
    GlobalOutput,
    Generic,
}

pub struct Id {
    value: Word,
    id_type: IdType,
}
impl Id {
    fn new(value: Word, id_type: IdType) -> Self {
        Self { value, id_type }
    }
}

pub enum FragmentInterfaceVariable {
    Input(def::PointerType),
    Output(def::PointerType),
    BuiltIn(def::FragmentBuiltIn),
}

impl<'a> Module<'a> {
    pub fn new_fragment_shader(
        allow_glsl_builtin_functions: bool,
        fragment_execution_modes: &[def::FragmentExecutionMode],
        interface_vars: &[FragmentInterfaceVariable],
    ) -> Result<(Self, Id, Vec<Id>), Error> {
        let mut capabilities = Capabilities::new(Capability::Shader);
        let mut ids = Ids::new();

        let glsl_builtin_functions = if allow_glsl_builtin_functions {
            Some(ids.get_next_id())
        } else {
            None
        };

        let memory_model = MemoryModel {
            addressing_model: def::AddressingModel::Logical,
            memory_model: def::MemoryModel::GLSL450,
        };

        let mut requried_capabilities = [Capability::Matrix; 5];
        let num_capabilities = memory_model
            .addressing_model
            .get_required_capabilities(&mut requried_capabilities);
        for c in requried_capabilities.iter().take(num_capabilities) {
            capabilities.add_capability(*c);
        }

        let num_capabilities = memory_model
            .memory_model
            .get_required_capabilities(&mut requried_capabilities);
        for c in requried_capabilities.iter().take(num_capabilities) {
            capabilities.add_capability(*c);
        }

        let mut global_declarations = GlobalDeclarations::new();

        let main_function_type = Global::TypeFunction(TypeFunctionDetails {
            return_type: def::ReturnType::Void,
            parameter_types: &MAIN_FUNCTION_PARAMETER_TYPES,
        });
        global_declarations.get_global_id(&mut ids, main_function_type);

        let main_function_id = ids.get_next_id();
        let mut main_function_parameters = Vec::with_capacity(MAIN_FUNCTION_PARAMETER_TYPES.len());
        for pt in MAIN_FUNCTION_PARAMETER_TYPES {
            main_function_parameters.push((pt.get_global(), ids.get_next_id()));
        }

        let functions = vec![Function {
            function_type: main_function_type,
            control: def::FunctionControl::None,
            id: main_function_id,
            parameters: main_function_parameters,
            label0_id: ids.get_next_id(),
            local_vars: Vec::new(),
            instructions: Vec::new(),
        }];

        // match execution_model {
        //     ExecutionModel::Vertex => {}
        //     ExecutionModel::Fragment => {}
        //     _ => return Err(Error::UnsupportedExecutionModel),
        // };
        let execution_model = ExecutionModel::Fragment;
        let num_capabilities =
            execution_model.get_required_capabilities(&mut requried_capabilities);
        for c in requried_capabilities.iter().take(num_capabilities) {
            capabilities.add_capability(*c);
        }

        let mut execution_modes = Vec::with_capacity(fragment_execution_modes.len());
        for fem in fragment_execution_modes {
            let em = def::ExecutionMode::Fragment(*fem);
            let num_capabilities = em.get_required_capabilities(&mut requried_capabilities);
            for c in requried_capabilities.iter().take(num_capabilities) {
                capabilities.add_capability(*c);
            }
            execution_modes.push(em);
        }

        let mut interface = Vec::with_capacity(interface_vars.len());
        let mut interface_ids = Vec::with_capacity(interface_vars.len());
        let mut decorations = Vec::new();
        for iv in interface_vars {
            match *iv {
                FragmentInterfaceVariable::Input(pointer_type) => {
                    let global = Global::Variable(VariableDetails {
                        pointer: def::TypePointerDetails {
                            storage_class: StorageClass::Input,
                            pointer_type,
                        },
                        initializer: None,
                    });
                    interface_ids.push(Id::new(
                        global_declarations.get_global_id(&mut ids, global),
                        IdType::GlobalInput,
                    ));
                    interface.push(global);
                }
                FragmentInterfaceVariable::Output(pointer_type) => {
                    let global = Global::Variable(VariableDetails {
                        pointer: def::TypePointerDetails {
                            storage_class: StorageClass::Output,
                            pointer_type,
                        },
                        initializer: None,
                    });
                    interface_ids.push(Id::new(
                        global_declarations.get_global_id(&mut ids, global),
                        IdType::GlobalOutput,
                    ));
                    interface.push(global);
                }
                FragmentInterfaceVariable::BuiltIn(frag_builtin) => {
                    let builtin = def::BuiltIn::Fragment(frag_builtin);
                    let (global, is_output) = builtin.get_global_and_is_output();
                    let global_id = global_declarations.get_global_id(&mut ids, global);
                    interface.push(global);
                    if is_output {
                        interface_ids.push(Id::new(global_id, IdType::GlobalOutput));
                    } else {
                        interface_ids.push(Id::new(global_id, IdType::GlobalInput));
                    }
                    decorations.push((Decoration::BuiltIn(builtin), global));
                }
            }
        }

        let entry_points = vec![EntryPoint::new(
            execution_model,
            main_function_id,
            "main",
            interface,
            execution_modes,
        )?];

        Ok((
            Self {
                capabilities,
                ids,
                glsl_builtin_functions,
                memory_model,
                entry_points,
                decorations,
                global_declarations,
                functions,
            },
            Id::new(main_function_id, IdType::Function),
            interface_ids,
        ))
    }

    pub fn add_global(&mut self, global: Global<'a>) -> Result<Id, Error> {
        Ok(Id::new(
            self.global_declarations
                .get_global_id(&mut self.ids, global),
            IdType::Generic,
        ))
    }

    pub fn new_function(&mut self, function_type: Global<'a>) -> Result<Id, Error> {
        match function_type {
            Global::TypeFunction(details) => {
                self.global_declarations
                    .get_global_id(&mut self.ids, function_type);
                let function_id = self.ids.get_next_id();
                let mut function_parameters = Vec::with_capacity(details.parameter_types.len());
                for pt in details.parameter_types {
                    function_parameters.push((pt.get_global(), self.ids.get_next_id()));
                }
                self.functions.push(Function {
                    function_type,
                    control: def::FunctionControl::None,
                    id: function_id,
                    parameters: function_parameters,
                    label0_id: self.ids.get_next_id(),
                    local_vars: Vec::new(),
                    instructions: Vec::new(),
                });
                Ok(Id::new(function_id, IdType::Function))
            }
            _ => Err(Error::InvalidGlobal),
        }
    }

    pub fn add_local_var(&mut self, _function_id: &Id) -> Result<(), Error> {
        Ok(())
    }

    pub fn get_word_stream(&self) -> Vec<u32> {
        let mut word_stream = def::create_word_stream_header(true, self.ids.get_id_bound());
        self.capabilities.add_words(&mut word_stream);
        if let Some(id) = self.glsl_builtin_functions {
            let instruction_extension_name = "GLSL.std.450";
            let additional_arguments = def::get_str_word_count(instruction_extension_name) - 1;
            word_stream.push(OpcodeName::ExtInstImport.get_word0(additional_arguments));
            word_stream.push(id);
            def::add_str_data(instruction_extension_name, &mut word_stream);
        }
        self.memory_model.add_words(&mut word_stream);
        for ep in &self.entry_points {
            ep.add_words(&mut word_stream, &self.global_declarations);
        }
        for ep in &self.entry_points {
            ep.add_execution_mode_words(&mut word_stream);
        }

        for (d, global) in &self.decorations {
            let global_id = self.global_declarations.check_global_id(global).unwrap();
            match d {
                Decoration::BuiltIn(b) => {
                    word_stream.push(OpcodeName::Decorate.get_word0(1));
                    word_stream.push(global_id);
                    word_stream.push(d.get_word());
                    word_stream.push(b.get_word());
                }
                Decoration::Location(l) => {
                    word_stream.push(OpcodeName::Decorate.get_word0(1));
                    word_stream.push(global_id);
                    word_stream.push(d.get_word());
                    word_stream.push(*l);
                }
                Decoration::DescriptorSet(s) => {
                    word_stream.push(OpcodeName::Decorate.get_word0(1));
                    word_stream.push(global_id);
                    word_stream.push(d.get_word());
                    word_stream.push(*s);
                }
                Decoration::Binding(b) => {
                    word_stream.push(OpcodeName::Decorate.get_word0(1));
                    word_stream.push(global_id);
                    word_stream.push(d.get_word());
                    word_stream.push(*b);
                }
                _ => panic!("Cannot currently handle decoration type!"),
            }
        }

        self.global_declarations.add_words(&mut word_stream);
        for fun in &self.functions {
            fun.add_words(&mut word_stream, &self.global_declarations);
        }
        word_stream
    }
}
