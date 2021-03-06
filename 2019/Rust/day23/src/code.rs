use crate::memory::Memory;

pub struct Computer {
    id: usize,
    context: ExecutionContext,
    input: Box<dyn Fn() -> Option<i64>>,
    output: Box<dyn Fn(i64, i64) -> ()>,
}

impl<'a> Computer {
    pub fn new(
        id: usize,
        memory: Memory,
        input: Box<dyn Fn() -> Option<i64>>,
        output: Box<dyn Fn(i64, i64) -> ()>,
    ) -> Computer {
        Computer {
            id,
            context: ExecutionContext::new(memory),
            input,
            output,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    fn read_input(&mut self) -> Option<i64> {
        //println!("Computer {} is reading its input", self.id);
        let read = (*self.input)();
        //println!("Computer {} read result: {:?}", self.id, read);
        read.or(Some(-1))
    }
    fn write_output(&mut self, addr: i64, value: i64) {
        //println!(
        //    "Computer {} is writing {} to address {}",
        //    self.id, value, addr
        //);
        (*self.output)(addr, value);
    }

    // pub fn execute(&mut self) -> ExecutionResult {
    //     //println!("Executing program; ip: {}", self.context.ip);
    //     let result = loop {
    //         match self.execute_single_instruction() {
    //             ExecutionResult::Executed => (),
    //             x => break x,
    //         };
    //     };

    //     //println!("Result: {:?}", result);
    //     result
    // }

    pub fn execute_single_instruction(&mut self) -> ExecutionResult {
        match self.context.read_op_code() {
            (OpCode::Add, parameter_modes) => {
                let (a, b, c) = self.context.extract_parameters3(parameter_modes);
                c.set(
                    a.get(&mut self.context) + b.get(&mut self.context),
                    &mut self.context,
                );
            }
            (OpCode::Mult, parameter_modes) => {
                let (a, b, c) = self.context.extract_parameters3(parameter_modes);
                c.set(
                    a.get(&self.context) * b.get(&self.context),
                    &mut self.context,
                );
            }
            (OpCode::Input, parameter_modes) => {
                match self.read_input() {
                    Some(value) => {
                        // println!("Reading input {}", value);
                        let a = self.context.extract_parameter(parameter_modes);
                        a.set(value, &mut self.context);
                    }
                    None => {
                        //println!("Halting program due to input read; ip: {}", context.ip);
                        // Revert the reading of the op-code, so we can read it again when the
                        // thread is resumed
                        self.context.ip -= 1;
                        return ExecutionResult::MoreInputNeeded;
                    }
                }
            }
            (OpCode::Output, parameter_modes) => {
                let a = self.context.extract_parameter(parameter_modes);
                let output = a.get(&self.context);
                let pending_outputs = &mut self.context.pending_outputs;
                pending_outputs.push(output);
                if pending_outputs.len() >= 3 {
                    let addr = pending_outputs[0];
                    let x = pending_outputs[1];
                    let y = pending_outputs[2];
                    pending_outputs.clear();

                    self.write_output(addr, x);
                    self.write_output(addr, y);
                }
            }
            (OpCode::JumpIfTrue, parameter_modes) => {
                let (a, b) = self.context.extract_parameters2(parameter_modes);
                if a.get(&self.context) != 0 {
                    let address = b.get(&self.context);
                    self.context.jump_to(address);
                }
            }
            (OpCode::JumpIfFalse, parameter_modes) => {
                let (a, b) = self.context.extract_parameters2(parameter_modes);
                if a.get(&self.context) == 0 {
                    let address = b.get(&self.context);
                    self.context.jump_to(address);
                }
            }
            (OpCode::LessThan, parameter_modes) => {
                let (a, b, c) = self.context.extract_parameters3(parameter_modes);
                let value = if a.get(&self.context) < b.get(&self.context) {
                    1
                } else {
                    0
                };
                c.set(value, &mut self.context);
            }
            (OpCode::Equals, parameter_modes) => {
                let (a, b, c) = self.context.extract_parameters3(parameter_modes);
                let value = if a.get(&self.context) == b.get(&self.context) {
                    1
                } else {
                    0
                };
                c.set(value, &mut self.context);
            }
            (OpCode::AdjustRelativeBase, parameter_modes) => {
                let a = self.context.extract_parameter(parameter_modes);
                let adjustment = a.get(&self.context);
                self.context.relative_base =
                    (self.context.relative_base as i64 + adjustment) as usize;
            }
            (OpCode::Exit, _) => {
                self.context.ended = true;
                return ExecutionResult::Exit;
            }
        };

        return ExecutionResult::Executed;
    }
}

enum OpCode {
    Add,
    Mult,
    Exit,
    Input,
    Output,
    JumpIfTrue,
    JumpIfFalse,
    LessThan,
    Equals,
    AdjustRelativeBase,
}

#[derive(Clone)]
struct ExecutionContext {
    ip: usize,
    memory: Memory,
    ended: bool,
    relative_base: usize,
    input: Vec<i64>,
    input_index: usize,
    output: i64,
    pending_outputs: Vec<i64>,
}

impl ExecutionContext {
    fn new(memory: Memory) -> ExecutionContext {
        ExecutionContext {
            ip: 0,
            memory,
            ended: false,
            relative_base: 0,
            output: 0,
            input_index: 0,
            input: vec![],
            pending_outputs: vec![],
        }
    }

    fn jump_to(&mut self, address: i64) {
        self.ip = address as usize;
    }

    fn read_op_code(&mut self) -> (OpCode, u32) {
        let value = self.memory[self.ip];
        let op_code_value = value % 100;
        let parameter_modes = (value / 100) as u32;

        let op_code = match op_code_value {
            1 => OpCode::Add,
            2 => OpCode::Mult,
            3 => OpCode::Input,
            4 => OpCode::Output,
            5 => OpCode::JumpIfTrue,
            6 => OpCode::JumpIfFalse,
            7 => OpCode::LessThan,
            8 => OpCode::Equals,
            9 => OpCode::AdjustRelativeBase,
            99 => OpCode::Exit,
            x => panic!("Unknown op code: {}; ip: {}", x, self.ip),
        };

        self.ip += 1;
        (op_code, parameter_modes)
    }
    fn extract_parameter(&mut self, parameter_modes: u32) -> Parameter {
        let mut param_modes = parameter_modes;
        self.get_parameter(&mut param_modes)
    }

    fn extract_parameters2(&mut self, parameter_modes: u32) -> (Parameter, Parameter) {
        let mut param_modes = parameter_modes;
        let x = self.get_parameter(&mut param_modes);
        let y = self.get_parameter(&mut param_modes);
        (x, y)
    }

    fn extract_parameters3(&mut self, parameter_modes: u32) -> (Parameter, Parameter, Parameter) {
        let mut param_modes = parameter_modes;
        let x = self.get_parameter(&mut param_modes);
        let y = self.get_parameter(&mut param_modes);
        let z = self.get_parameter(&mut param_modes);
        (x, y, z)
    }

    fn get_parameter(&mut self, parameter_modes: &mut u32) -> Parameter {
        // Get the parameter mode for this parameter
        let parameter_mode = match *parameter_modes % 10 {
            0 => ParameterMode::Position,
            1 => ParameterMode::Immediate,
            2 => ParameterMode::Relative,
            x => panic!(format!("Incorrect parameter mode: {}", x)),
        };
        *parameter_modes /= 10;

        let parameter_value = self.memory[self.ip];
        self.ip += 1;

        match parameter_mode {
            ParameterMode::Position => Parameter::Reference(parameter_value as usize),
            ParameterMode::Immediate => Parameter::ImmediateValue(parameter_value),
            ParameterMode::Relative => {
                let address = (parameter_value + self.relative_base as i64) as usize;
                Parameter::Reference(address)
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ExecutionResult {
    Executed,
    MoreInputNeeded,
    Exit,
}

enum Parameter {
    ImmediateValue(i64),
    Reference(usize),
}

impl<'a> Parameter {
    fn get(&self, context: &ExecutionContext) -> i64 {
        match self {
            Parameter::Reference(address) => context.memory[*address],
            Parameter::ImmediateValue(value) => *value,
        }
    }

    fn set(&self, value: i64, context: &mut ExecutionContext) -> () {
        match self {
            Parameter::Reference(address) => context.memory[*address] = value,
            Parameter::ImmediateValue(value) => panic!(format!(
                "Attempted to write value {} to an immediate parameter",
                value
            )),
        }
    }
}

enum ParameterMode {
    Position,
    Immediate,
    Relative,
}
