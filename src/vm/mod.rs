pub mod bytecode;

use crate::error::{Result, VenturiError};
use bytecode::{Instruction, Value};
use std::collections::HashMap;

pub struct ExecContext {
    pub variables: HashMap<String, Value>,
    pub current_van: Option<String>,
    pub node_outputs: HashMap<String, Value>,
}

impl ExecContext {
    pub fn new(van: Option<String>) -> Self {
        ExecContext {
            variables: HashMap::new(),
            current_van: van,
            node_outputs: HashMap::new(),
        }
    }

    pub fn with_inputs(mut self, inputs: HashMap<String, Value>) -> Self {
        for (k, v) in inputs {
            self.variables.insert(k, v);
        }
        self
    }
}

struct CatchFrame {
    error_type: String,
    handler_ip: usize,
    stack_depth: usize,
}

pub struct Vm;

impl Vm {
    pub fn new() -> Self {
        Vm
    }

    pub fn execute(&self, code: &[Instruction], ctx: &mut ExecContext) -> Result<Value> {
        let mut stack: Vec<Value> = Vec::new();
        let mut ip = 0usize;
        let mut catchers: Vec<CatchFrame> = Vec::new();

        while ip < code.len() {
            let instr = &code[ip];
            ip += 1;

            match instr {
                Instruction::Push(v) => {
                    stack.push(v.clone());
                }

                Instruction::Pop => {
                    stack.pop();
                }

                Instruction::Dup => {
                    if let Some(top) = stack.last().cloned() {
                        stack.push(top);
                    }
                }

                Instruction::Load(name) => {
                    let val = ctx
                        .variables
                        .get(name)
                        .or_else(|| ctx.node_outputs.get(name))
                        .cloned()
                        .unwrap_or(Value::Null);
                    stack.push(val);
                }

                Instruction::Store(name) => {
                    let val = stack.pop().unwrap_or(Value::Null);
                    ctx.variables.insert(name.clone(), val);
                }

                Instruction::Call(name, arg_count) => {
                    let mut args = Vec::new();
                    for _ in 0..*arg_count {
                        args.push(stack.pop().unwrap_or(Value::Null));
                    }
                    args.reverse();
                    let result = self.call_builtin(name, args, ctx)?;
                    stack.push(result);
                }

                Instruction::CallNode(name) => {
                    let val = ctx
                        .node_outputs
                        .get(name)
                        .cloned()
                        .unwrap_or(Value::Null);
                    stack.push(val);
                }

                Instruction::Return => {
                    return Ok(stack.pop().unwrap_or(Value::Null));
                }

                Instruction::Add => {
                    let b = stack.pop().unwrap_or(Value::Null);
                    let a = stack.pop().unwrap_or(Value::Null);
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => stack.push(Value::Int(x + y)),
                        (Value::Float(x), Value::Float(y)) => stack.push(Value::Float(x + y)),
                        (Value::Str(x), Value::Str(y)) => {
                            stack.push(Value::Str(format!("{}{}", x, y)))
                        }
                        _ => stack.push(Value::Null),
                    }
                }

                Instruction::Sub => {
                    let b = stack.pop().unwrap_or(Value::Null);
                    let a = stack.pop().unwrap_or(Value::Null);
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => stack.push(Value::Int(x - y)),
                        (Value::Float(x), Value::Float(y)) => stack.push(Value::Float(x - y)),
                        _ => stack.push(Value::Null),
                    }
                }

                Instruction::Mul => {
                    let b = stack.pop().unwrap_or(Value::Null);
                    let a = stack.pop().unwrap_or(Value::Null);
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => stack.push(Value::Int(x * y)),
                        (Value::Float(x), Value::Float(y)) => stack.push(Value::Float(x * y)),
                        _ => stack.push(Value::Null),
                    }
                }

                Instruction::Div => {
                    let b = stack.pop().unwrap_or(Value::Null);
                    let a = stack.pop().unwrap_or(Value::Null);
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) if y != 0 => {
                            stack.push(Value::Int(x / y))
                        }
                        (Value::Float(x), Value::Float(y)) if y != 0.0 => {
                            stack.push(Value::Float(x / y))
                        }
                        _ => stack.push(Value::Null),
                    }
                }

                Instruction::Eq => {
                    let b = stack.pop().unwrap_or(Value::Null);
                    let a = stack.pop().unwrap_or(Value::Null);
                    stack.push(Value::Bool(a == b));
                }

                Instruction::Neq => {
                    let b = stack.pop().unwrap_or(Value::Null);
                    let a = stack.pop().unwrap_or(Value::Null);
                    stack.push(Value::Bool(a != b));
                }

                Instruction::Lt => {
                    let b = stack.pop().unwrap_or(Value::Null);
                    let a = stack.pop().unwrap_or(Value::Null);
                    let result = match (a, b) {
                        (Value::Int(x), Value::Int(y)) => x < y,
                        (Value::Float(x), Value::Float(y)) => x < y,
                        _ => false,
                    };
                    stack.push(Value::Bool(result));
                }

                Instruction::Gt => {
                    let b = stack.pop().unwrap_or(Value::Null);
                    let a = stack.pop().unwrap_or(Value::Null);
                    let result = match (a, b) {
                        (Value::Int(x), Value::Int(y)) => x > y,
                        (Value::Float(x), Value::Float(y)) => x > y,
                        _ => false,
                    };
                    stack.push(Value::Bool(result));
                }

                Instruction::Jump(target) => {
                    ip = *target;
                }

                Instruction::JumpIfFalse(target) => {
                    let cond = stack.pop().unwrap_or(Value::Null);
                    if !cond.truthy() {
                        ip = *target;
                    }
                }

                Instruction::Nop => {}

                Instruction::PushCatcher(error_type, handler_ip) => {
                    catchers.push(CatchFrame {
                        error_type: error_type.clone(),
                        handler_ip: *handler_ip,
                        stack_depth: stack.len(),
                    });
                }

                Instruction::PopCatcher => {
                    catchers.pop();
                }

                Instruction::Throw(msg) => {
                    // Find matching catcher
                    if let Some(frame) = catchers.pop() {
                        // Restore stack to catcher's depth
                        stack.truncate(frame.stack_depth);
                        // Push the error message as binding
                        stack.push(Value::Str(msg.clone()));
                        ip = frame.handler_ip;
                    } else {
                        return Err(VenturiError::Vm(msg.clone()));
                    }
                }

                Instruction::WrapOk => {
                    let val = stack.pop().unwrap_or(Value::Null);
                    stack.push(Value::Ok(Box::new(val)));
                }

                Instruction::WrapErr => {
                    let val = stack.pop().unwrap_or(Value::Null);
                    let msg = match val {
                        Value::Str(s) => s,
                        other => format!("{}", other),
                    };
                    stack.push(Value::Err(msg));
                }

                Instruction::CheckVan(required) => {
                    let allowed = ctx
                        .current_van
                        .as_deref()
                        .map(|v| v == required)
                        .unwrap_or(false);
                    if !allowed {
                        return Err(VenturiError::Permission {
                            required: required.clone(),
                            got: ctx.current_van.clone(),
                        });
                    }
                }

                Instruction::Log => {
                    let val = stack.pop().unwrap_or(Value::Null);
                    eprintln!("[venturi log] {}", val);
                    stack.push(Value::Null);
                }
            }
        }

        Ok(stack.pop().unwrap_or(Value::Null))
    }

    fn call_builtin(&self, name: &str, args: Vec<Value>, ctx: &mut ExecContext) -> Result<Value> {
        match name {
            "log" => {
                let msg = args.first().map(|v| format!("{}", v)).unwrap_or_default();
                eprintln!("[venturi log] {}", msg);
                Ok(Value::Null)
            }
            "clean" => {
                // Identity transform — in a real implementation, clean data
                Ok(args.into_iter().next().unwrap_or(Value::Null))
            }
            "normalize" => {
                Ok(args.into_iter().next().unwrap_or(Value::Null))
            }
            "scale" => {
                Ok(args.into_iter().next().unwrap_or(Value::Null))
            }
            "validate" => {
                Ok(args.into_iter().next().unwrap_or(Value::Null))
            }
            "enrich" => {
                Ok(args.into_iter().next().unwrap_or(Value::Null))
            }
            "extract_features" => {
                Ok(args.into_iter().next().unwrap_or(Value::Null))
            }
            "notify_service" => {
                let msg = args.first().map(|v| format!("{}", v)).unwrap_or_default();
                eprintln!("[venturi notify] {}", msg);
                Ok(Value::Bool(true))
            }
            "log_event" => {
                let val = args.first().map(|v| format!("{}", v)).unwrap_or_default();
                eprintln!("[venturi event] {}", val);
                Ok(Value::Null)
            }
            "api_send" => {
                Ok(Value::Bool(true))
            }
            other => {
                // Check if it's a node in node_outputs
                if let Some(val) = ctx.node_outputs.get(other).cloned() {
                    return Ok(val);
                }
                // Unknown call: return Null
                eprintln!("[venturi warn] Unknown call: {}({} args)", other, args.len());
                Ok(Value::Null)
            }
        }
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}
