use crate::memory::Memory;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::io::{stdin, stdout, Read};
use std::thread::sleep;
use std::time::Duration;

#[cfg(unix)]
extern crate ncurses;

mod memory;

type Result<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
struct Pos(i32, i32);

enum Cell {
    Robot(RobotStatus),
    Empty,
    Scaffold,
}

impl Cell {
    fn is_scaffold(&self) -> bool {
        match self {
            Cell::Scaffold => true,
            _ => false,
        }
    }
}

enum RobotStatus {
    Up,
    Down,
    Left,
    Right,
    Falling,
}

fn main() -> Result<()> {
    let file_name = env::args().nth(1).expect("Enter a file name");

    let mut instructions = String::new();
    File::open(file_name)?
        .read_to_string(&mut instructions)
        .expect("Failed to read input file");

    init();
    let memory = Memory::parse(&instructions);

    let mut context = ExecutionContext::new(&memory);
    loop {
        match execute_program(&mut context) {
            ExecutionResult::Exit => break,
            ExecutionResult::MoreInputNeeded => {
                print!("Input: ");
                stdout().flush().unwrap();
                let mut input = String::new();
                stdin().read_line(&mut input).unwrap();
                context.input = input.replace("\r", "");
                context.input_index = 0;
            }
        }
    }

    Ok(())
}

fn build_grid(chars: &Vec<i32>) -> HashMap<Pos, Cell> {
    let mut map = HashMap::new();
    let mut x = 0;
    let mut y = 0;

    for v in chars {
        match v {
            10 => {
                y += 1;
                x = 0;
            }
            c => {
                let cell = match *c as u8 as char {
                    '.' => Cell::Empty,
                    '#' => Cell::Scaffold,
                    x => parse_robot_cell(x),
                };

                map.insert(Pos(x, y), cell);
                x += 1;
            }
        }
    }

    map
}

fn parse_robot_cell(ch: char) -> Cell {
    let status = match ch {
        '^' => RobotStatus::Up,
        '>' => RobotStatus::Left,
        'v' => RobotStatus::Down,
        '<' => RobotStatus::Right,
        'X' => RobotStatus::Falling,
        x => panic!("Unknown char: {}", x),
    };
    Cell::Robot(status)
}

fn draw_grid(chars: &Vec<i32>) {
    clear();

    for ch in chars {
        match ch {
            10 => println(&""),
            c => print(&format!("{}", *c as u8 as char)),
        }
    }
    println("");
    refresh();
    sleep(Duration::from_millis(20));
}

#[derive(Clone)]
struct ExecutionContext {
    ip: usize,
    memory: Memory,
    ended: bool,
    relative_base: usize,
    input: String,
    input_index: usize,
    output: Vec<i32>,
}

impl ExecutionContext {
    fn new(memory: &Memory) -> ExecutionContext {
        ExecutionContext {
            ip: 0,
            memory: memory.clone(),
            ended: false,
            relative_base: 0,
            output: vec![],
            input_index: 0,
            input: String::new(),
        }
    }

    fn read_input(&mut self) -> Option<i64> {
        let index = self.input_index;
        self.input_index += 1;
        let res = self.input.chars().nth(index).map(|x| x as i64);

        //println!("Reading input: {:?}", res);
        res
    }

    fn write_output(&mut self, value: i64) {
        //println!("{}", value);
        if value > 128 {
            println!("Result: {}", value);
            return;
        }
        print!("{}", value as u8 as char);
        self.output.push(value as i32);
        // if value == 10 && self.output[self.output.len() - 2] == 10 {
        //     set_cursor_position(0, 0);
        //     //sleep(Duration::from_millis(0));
        // }
        //self.output.clear();
    }
}

enum ExecutionResult {
    MoreInputNeeded,
    Exit,
}

fn execute_program(context: &mut ExecutionContext) -> ExecutionResult {
    // println!("Executing program; ip: {}", context.ip.get());
    loop {
        match read_op_code(context) {
            (OpCode::Add, parameter_modes) => {
                let (a, b, c) = extract_parameters3(context, parameter_modes);
                c.set(a.get(context) + b.get(context), context);
            }
            (OpCode::Mult, parameter_modes) => {
                let (a, b, c) = extract_parameters3(context, parameter_modes);
                c.set(a.get(context) * b.get(context), context);
            }
            (OpCode::Input, parameter_modes) => {
                match context.read_input() {
                    Some(value) => {
                        //println!("Reading input {}", value);
                        let a = extract_parameter(context, parameter_modes);
                        a.set(value, context);
                    }
                    None => {
                        //println!("Halting program due to input read; ip: {}", context.ip);
                        // Revert the reading of the op-code, so we can read it again when the
                        // thread is resumed
                        context.ip -= 1;
                        return ExecutionResult::MoreInputNeeded;
                    }
                }
            }
            (OpCode::Output, parameter_modes) => {
                let a = extract_parameter(context, parameter_modes);
                let output = a.get(&context);
                //println!("Output: {}", output);
                context.write_output(output);
            }
            (OpCode::JumpIfTrue, parameter_modes) => {
                let (a, b) = extract_parameters2(context, parameter_modes);
                if a.get(&context) != 0 {
                    let address = b.get(&context);
                    jump_to(&mut context.ip, address);
                }
            }
            (OpCode::JumpIfFalse, parameter_modes) => {
                let (a, b) = extract_parameters2(context, parameter_modes);
                if a.get(&context) == 0 {
                    let address = b.get(&context);
                    jump_to(&mut context.ip, address);
                }
            }
            (OpCode::LessThan, parameter_modes) => {
                let (a, b, c) = extract_parameters3(context, parameter_modes);
                let value = if a.get(&context) < b.get(&context) {
                    1
                } else {
                    0
                };
                c.set(value, context);
            }
            (OpCode::Equals, parameter_modes) => {
                let (a, b, c) = extract_parameters3(context, parameter_modes);
                let value = if a.get(&context) == b.get(&context) {
                    1
                } else {
                    0
                };
                c.set(value, context);
            }
            (OpCode::AdjustRelativeBase, parameter_modes) => {
                let a = extract_parameter(context, parameter_modes);
                let adjustment = a.get(&context);
                context.relative_base = (context.relative_base as i64 + adjustment) as usize;
            }
            (OpCode::Exit, _) => {
                println!("Exiting");
                context.ended = true;
                return ExecutionResult::Exit;
            }
        }

        // println!("Values: {:?}", memory);
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

fn jump_to(ip: &mut usize, address: i64) {
    *ip = address as usize;
}

fn read_op_code(context: &mut ExecutionContext) -> (OpCode, u32) {
    let value = context.memory[context.ip];
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
        x => panic!("Unknown op code: {}; ip: {}", x, context.ip),
    };

    context.ip += 1;
    (op_code, parameter_modes)
}

fn extract_parameter(context: &mut ExecutionContext, parameter_modes: u32) -> Parameter {
    let mut param_modes = parameter_modes;
    get_parameter(context, &mut param_modes)
}

fn extract_parameters2(
    context: &mut ExecutionContext,
    parameter_modes: u32,
) -> (Parameter, Parameter) {
    let mut param_modes = parameter_modes;
    let x = get_parameter(context, &mut param_modes);
    let y = get_parameter(context, &mut param_modes);
    (x, y)
}

fn extract_parameters3(
    context: &mut ExecutionContext,
    parameter_modes: u32,
) -> (Parameter, Parameter, Parameter) {
    let mut param_modes = parameter_modes;
    let x = get_parameter(context, &mut param_modes);
    let y = get_parameter(context, &mut param_modes);
    let z = get_parameter(context, &mut param_modes);
    (x, y, z)
}

fn get_parameter(context: &mut ExecutionContext, parameter_modes: &mut u32) -> Parameter {
    // Get the parameter mode for this parameter
    let parameter_mode = match *parameter_modes % 10 {
        0 => ParameterMode::Position,
        1 => ParameterMode::Immediate,
        2 => ParameterMode::Relative,
        x => panic!(format!("Incorrect parameter mode: {}", x)),
    };
    *parameter_modes /= 10;

    let parameter_value = context.memory[context.ip];
    context.ip += 1;

    match parameter_mode {
        ParameterMode::Position => Parameter::Reference(parameter_value as usize),
        ParameterMode::Immediate => Parameter::ImmediateValue(parameter_value),
        ParameterMode::Relative => {
            let address = (parameter_value + context.relative_base as i64) as usize;
            Parameter::Reference(address)
        }
    }
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

extern crate kernel32;
extern crate winapi;

#[cfg(windows)]
use winapi::wincon::CONSOLE_SCREEN_BUFFER_INFO;
#[cfg(windows)]
use winapi::wincon::COORD;
#[cfg(windows)]
use winapi::wincon::SMALL_RECT;
#[cfg(windows)]
use winapi::DWORD;
#[cfg(windows)]
use winapi::HANDLE;
#[cfg(windows)]
use winapi::WORD;

#[cfg(windows)]
static mut CONSOLE_HANDLE: Option<HANDLE> = None;

#[cfg(windows)]
fn get_output_handle() -> HANDLE {
    unsafe {
        if let Some(handle) = CONSOLE_HANDLE {
            return handle;
        } else {
            let handle = kernel32::GetStdHandle(winapi::STD_OUTPUT_HANDLE);
            CONSOLE_HANDLE = Some(handle);
            return handle;
        }
    }
}

#[cfg(windows)]
fn get_buffer_info() -> winapi::CONSOLE_SCREEN_BUFFER_INFO {
    let handle = get_output_handle();
    if handle == winapi::INVALID_HANDLE_VALUE {
        panic!("NoConsole")
    }
    let mut buffer = CONSOLE_SCREEN_BUFFER_INFO {
        dwSize: COORD { X: 0, Y: 0 },
        dwCursorPosition: COORD { X: 0, Y: 0 },
        wAttributes: 0 as WORD,
        srWindow: SMALL_RECT {
            Left: 0,
            Top: 0,
            Right: 0,
            Bottom: 0,
        },
        dwMaximumWindowSize: COORD { X: 0, Y: 0 },
    };
    unsafe {
        kernel32::GetConsoleScreenBufferInfo(handle, &mut buffer);
    }
    buffer
}

#[cfg(windows)]
fn init() {}

#[cfg(unix)]
fn init() {
    ncurses::initscr();
}

#[cfg(unix)]
fn clear() {
    //ncurses::clear();
    ncurses::mv(0, 0);
}

#[cfg(windows)]
fn print(msg: &str) {
    print!("{}", msg);
}

#[cfg(unix)]
fn print(msg: &str) {
    ncurses::printw(msg);
}

#[cfg(windows)]
fn println(msg: &str) {
    println!("{}", msg);
}

#[cfg(unix)]
fn println(msg: &str) {
    ncurses::addstr(msg);
    ncurses::addstr("\n");
}

#[cfg(windows)]
fn refresh() {}

#[cfg(unix)]
fn refresh() {
    ncurses::refresh();
}

#[cfg(windows)]
fn clear() {
    let handle = get_output_handle();
    if handle == winapi::INVALID_HANDLE_VALUE {
        panic!("NoConsole")
    }

    let screen_buffer = get_buffer_info();
    let console_size: DWORD = screen_buffer.dwSize.X as u32 * screen_buffer.dwSize.Y as u32;
    let coord_screen = COORD { X: 0, Y: 0 };

    let mut amount_chart_written: DWORD = 0;
    unsafe {
        kernel32::FillConsoleOutputCharacterW(
            handle,
            32 as winapi::WCHAR,
            console_size,
            coord_screen,
            &mut amount_chart_written,
        );
    }
    set_cursor_position(0, 0);
}

#[cfg(windows)]
fn set_cursor_position(y: i16, x: i16) {
    let handle = get_output_handle();
    if handle == winapi::INVALID_HANDLE_VALUE {
        panic!("NoConsole")
    }
    unsafe {
        kernel32::SetConsoleCursorPosition(handle, COORD { X: x, Y: y });
    }
}
