use byteorder::BigEndian;
use byteorder::ByteOrder;
use std::collections::HashMap;
use std::rc::Rc;

use crate::code::definitions::*;
use crate::code::opcode::Opcode;
use crate::common::error::RTError;
use crate::common::object::Array;
use crate::common::object::HMap;
use crate::common::object::Object;

const STACK_SIZE: usize = 4096;
pub const GLOBALS_SIZE: usize = 65536;

/*
 * The virtual machine has the constants and instructions generated by the
 * compiler and has a stack. The stack pointer always points to the next
 * available free slot. So, the top of stack is stack[len - 1]. stack pointer
 * is assumed to be '0' when stack is empt and stack_top() would return Nil.
 */
pub struct VM {
    constants: Vec<Object>,
    stack: Vec<Rc<Object>>,
    sp: usize,
    pub globals: Vec<Rc<Object>>,
}

enum BinaryOperation {
    Add,
    Sub,
    Mul,
    Div,
    Greater,
}

impl VM {
    pub fn new(constants: Vec<Object>) -> VM {
        let data = Rc::new(Object::Nil);
        VM {
            constants,
            stack: Vec::with_capacity(STACK_SIZE),
            sp: 0,
            globals: vec![data; GLOBALS_SIZE],
        }
    }

    pub fn new_with_global_store(constants: Vec<Object>, globals: Vec<Rc<Object>>) -> VM {
        let mut vm = VM::new(constants);
        vm.globals = globals;
        vm
    }

    pub fn peek(&self, distance: usize) -> Rc<Object> {
        if self.sp - distance == 0 {
            Rc::new(Object::Nil)
        } else {
            Rc::clone(&self.stack[self.sp - distance - 1])
        }
    }

    /*
     * If there isn't enough space on stack then push elements on to it
     * Otherwise, set the element on stack based on the stack pointer (sp).
     * In either case, increment 'sp' to point to the newly available slot.
     */
    pub fn push(&mut self, obj: Rc<Object>) {
        if self.sp >= self.stack.len() {
            self.stack.push(obj);
        } else {
            self.stack[self.sp] = obj;
        }
        self.sp += 1;
    }

    pub fn pop(&mut self, line: usize) -> Result<Rc<Object>, RTError> {
        if self.sp == 0 {
            return Err(RTError::new("Stack underflow!", line));
        }
        let obj = self.stack[self.sp - 1].clone();
        self.sp -= 1;
        Ok(obj)
    }

    pub fn last_popped(&mut self) -> Rc<Object> {
        self.stack[self.sp].clone()
    }

    /*
     * The main run loop for the interpreter. Since this is the hot path,
     * do not use functions such as lookup() or read_operands() for decoding
     * instructions and operands.
     */
    pub fn run(&mut self, instructions: &Instructions) -> Result<(), RTError> {
        let mut ip = 0;
        while ip < instructions.len() {
            let op = Opcode::from(instructions.code[ip]);
            let line = instructions.lines[ip];
            match op {
                Opcode::Constant => {
                    let const_index =
                        BigEndian::read_u16(&instructions.code[ip + 1..ip + 3]) as usize;
                    let constant = self
                        .constants
                        .get(const_index)
                        .ok_or_else(|| RTError::new("constant not found", line))?;
                    self.push(Rc::new(constant.clone()));
                    // skip over the two bytes of the operand in the next cycle
                    ip += 2;
                }
                Opcode::Pop => {
                    self.pop(line)?;
                }
                Opcode::Add => {
                    self.binary_op(BinaryOperation::Add, |a, b| a + b, line)?;
                }
                Opcode::Sub => {
                    self.binary_op(BinaryOperation::Sub, |a, b| a - b, line)?;
                }
                Opcode::Mul => {
                    self.binary_op(BinaryOperation::Mul, |a, b| a * b, line)?;
                }
                Opcode::Div => {
                    self.binary_op(BinaryOperation::Div, |a, b| a / b, line)?;
                }
                Opcode::True => self.push(Rc::new(Object::Bool(true))),
                Opcode::False => self.push(Rc::new(Object::Bool(false))),
                Opcode::Equal => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    self.push(Rc::new(Object::Bool(a.as_ref() == b.as_ref())));
                }
                Opcode::NotEqual => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    self.push(Rc::new(Object::Bool(a != b)));
                }
                Opcode::Greater => {
                    self.binary_op(BinaryOperation::Greater, |a, b| Object::Bool(a > b), line)?;
                }
                Opcode::Minus => {
                    if !self.peek(0).is_number() {
                        return Err(RTError::new("Operand must be a number", line));
                    }
                    let obj = self.pop(line)?.clone();
                    let val = -&*obj;
                    self.push(Rc::new(val));
                }
                Opcode::Bang => {
                    let obj = self.pop(line)?;
                    self.push(Rc::new(Object::Bool(obj.is_falsey())));
                }
                Opcode::Jump => {
                    let bytes = &instructions.code[ip + 1..ip + 3];
                    // decode the operand (jump address) right after the opcode
                    ip = u16::from_be_bytes([bytes[0], bytes[1]]) as usize - 1;
                }
                Opcode::JumpIfFalse => {
                    let bytes = &instructions.code[ip + 1..ip + 3];
                    // decode the operand (jump address) right after the opcode
                    let pos: usize = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
                    // skip over the two bytes of the operand in the next cycle
                    ip += 2;
                    let condition = self.pop(line)?;
                    if condition.is_falsey() {
                        ip = pos - 1;
                    }
                }
                Opcode::Nil => {
                    self.push(Rc::new(Object::Nil));
                }
                Opcode::GetGlobal => {
                    let bytes = &instructions.code[ip + 1..ip + 3];
                    // decode the operand (index to globals)
                    let globals_index: usize = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
                    ip += 2;
                    self.push(self.globals[globals_index].clone());
                }
                Opcode::SetGlobal => {
                    let bytes = &instructions.code[ip + 1..ip + 3];
                    // decode the operand (index to globals)
                    let globals_index: usize = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
                    ip += 2;
                    self.globals[globals_index] = self.pop(line)?;
                }
                Opcode::Array => {
                    // Read the first operand i.e. the number of array elements
                    let num_elements = BigEndian::read_u16(&instructions.code[ip + 1..]) as usize;
                    let elements = self.build_array(self.sp - num_elements, self.sp);
                    // pop 'num_elements' off the stack
                    self.sp -= num_elements;
                    // Push the array back onto the stack as an object
                    self.push(Rc::new(Object::Arr(Array { elements })));
                    // skip over the two bytes of the operand in the next cycle
                    ip += 2;
                }
                Opcode::Map => {
                    // Read the first operand i.e. the number of pairs
                    let num_elements =
                        BigEndian::read_u16(&instructions.code[ip + 1..ip + 3]) as usize;
                    let pairs = self.build_map(self.sp - num_elements, self.sp);
                    // pop 'num_elements' off the stack
                    self.sp -= num_elements;
                    // Push the array back onto the stack as an object
                    self.push(Rc::new(Object::Map(HMap { pairs })));
                    // skip over the two bytes of the operand in the next cycle
                    ip += 2;
                }
                Opcode::Invalid => {
                    return Err(RTError::new(
                        &format!("opcode {} undefined", op as u8),
                        line,
                    ))
                }
            }
            ip += 1;
        }

        Ok(())
    }

    fn binary_op(
        &mut self,
        optype: BinaryOperation,
        op: fn(a: &Object, b: &Object) -> Object,
        line: usize,
    ) -> Result<(), RTError> {
        // pop right before left
        let right = self.pop(line)?;
        let left = self.pop(line)?;

        match (&*left, &*right) {
            (Object::Number(_), Object::Number(_)) => {
                self.push(Rc::new(op(&left, &right)));
                Ok(())
            }
            (Object::Str(left), Object::Str(right)) => {
                if matches!(optype, BinaryOperation::Add) {
                    self.push(Rc::new(Object::Str(format!("{}{}", left, right))));
                    Ok(())
                } else {
                    Err(RTError::new("Invalid operation on strings.", line))
                }
            }
            (Object::Str(s), Object::Number(n)) | (Object::Number(n), Object::Str(s)) => {
                if matches!(optype, BinaryOperation::Mul) {
                    self.push(Rc::new(Object::Str(s.repeat(*n as usize))));
                    Ok(())
                } else {
                    Err(RTError::new("Invalid operation on strings.", line))
                }
            }
            _ => Err(RTError::new("Invalid binary operation.", line)),
        }
    }

    // Build array from elements on stack
    fn build_array(&self, start_index: usize, end_index: usize) -> Vec<Rc<Object>> {
        let mut elements = Vec::with_capacity(end_index - start_index);
        for i in start_index..end_index {
            elements.push(self.stack[i].clone());
        }
        elements
    }

    // Build map from objects on stack
    fn build_map(&self, start_index: usize, end_index: usize) -> HashMap<Rc<Object>, Rc<Object>> {
        let mut elements = HashMap::with_capacity(end_index - start_index);
        for i in (start_index..end_index).step_by(2) {
            let key = self.stack[i].clone();
            let val = self.stack[i + 1].clone();
            elements.insert(key, val);
        }
        elements
    }
}
