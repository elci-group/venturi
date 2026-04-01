use crate::ast::{CatchClause, Expr, FuncDef, Stmt};
use crate::vm::bytecode::{Instruction, Value};

pub struct Compiler {
    code: Vec<Instruction>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler { code: Vec::new() }
    }

    pub fn compile_func(mut self, func: &FuncDef) -> Vec<Instruction> {
        // Load parameters from variables (they'll be pre-stored by the runtime)
        self.compile_block(&func.body);

        // Ensure there's always a return
        if !matches!(self.code.last(), Some(Instruction::Return)) {
            self.code.push(Instruction::Push(Value::Null));
            self.code.push(Instruction::Return);
        }

        self.code
    }

    fn compile_block(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.compile_stmt(stmt);
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Assign(name, expr) => {
                self.compile_expr(expr);
                self.code.push(Instruction::Store(name.clone()));
            }

            Stmt::Return(expr) => {
                self.compile_expr(expr);
                self.code.push(Instruction::Return);
            }

            Stmt::TryCatch { body, catches } => {
                self.compile_try_catch(body, catches);
            }

            Stmt::Expr(expr) => {
                self.compile_expr(expr);
                // Discard result of expression statement
                self.code.push(Instruction::Pop);
            }
        }
    }

    fn compile_try_catch(&mut self, body: &[Stmt], catches: &[CatchClause]) {
        // For each catch clause, emit a PushCatcher before the try body.
        // We'll use a two-pass approach: emit placeholder jumps and fix them up.

        // We only support one catch level for now; emit PushCatcher for each catch
        // The handler_ip will be patched after we know the body length.

        // Reserve PushCatcher slots
        let mut catcher_placeholders: Vec<(String, usize)> = Vec::new();

        for catch in catches {
            let placeholder_ip = self.code.len();
            self.code
                .push(Instruction::PushCatcher(catch.error_type.clone(), 0));
            catcher_placeholders.push((catch.error_type.clone(), placeholder_ip));
        }

        // Compile body
        self.compile_block(body);

        // After body succeeds, pop all catchers and jump past handlers
        for _ in catches {
            self.code.push(Instruction::PopCatcher);
        }

        // Jump past all catch handlers
        let jump_past_placeholder = self.code.len();
        self.code.push(Instruction::Jump(0)); // patch later

        // Compile each catch handler
        let mut handler_starts = Vec::new();
        for catch in catches {
            let handler_start = self.code.len();
            handler_starts.push(handler_start);

            // The error message is on the stack as a Str (pushed by Throw)
            // Store it as the catch binding
            self.code.push(Instruction::Store(catch.binding.clone()));

            self.compile_block(&catch.body);

            // After catch body, jump to end
            self.code.push(Instruction::Jump(0)); // patch later
        }

        let end_ip = self.code.len();

        // Patch jump-past-handlers
        self.code[jump_past_placeholder] = Instruction::Jump(end_ip);

        // Patch each catch's end-jump to end_ip
        // The jumps-to-end are at positions after each handler body
        // We need to track them. Let's re-scan for the second set of Jump(0)s
        // Actually, let's redo with explicit tracking.

        // Patch PushCatcher handler_ip to point to their handlers
        for (i, (_etype, placeholder_ip)) in catcher_placeholders.iter().enumerate() {
            if let Some(&handler_start) = handler_starts.get(i) {
                self.code[*placeholder_ip] =
                    Instruction::PushCatcher(catches[i].error_type.clone(), handler_start);
            }
        }

        // Patch handler end-jumps (the Jump(0) at end of each handler)
        // These are at jump_past_placeholder+1 + offset for each handler
        // This is complex with variable-length handlers. Instead, let's find all Jump(0) after jump_past_placeholder
        for ip in (jump_past_placeholder + 1)..end_ip {
            if let Instruction::Jump(0) = self.code[ip] {
                self.code[ip] = Instruction::Jump(end_ip);
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::IntLit(n) => {
                self.code.push(Instruction::Push(Value::Int(*n)));
            }
            Expr::FloatLit(f) => {
                self.code.push(Instruction::Push(Value::Float(*f)));
            }
            Expr::StrLit(s) => {
                self.code.push(Instruction::Push(Value::Str(s.clone())));
            }
            Expr::BoolLit(b) => {
                self.code.push(Instruction::Push(Value::Bool(*b)));
            }
            Expr::Ident(name) => {
                self.code.push(Instruction::Load(name.clone()));
            }
            Expr::Call(name, args) => {
                // Handle log specially
                if name == "log" {
                    for arg in args {
                        self.compile_expr(arg);
                    }
                    self.code.push(Instruction::Log);
                } else {
                    for arg in args {
                        self.compile_expr(arg);
                    }
                    self.code.push(Instruction::Call(name.clone(), args.len()));
                }
            }
            Expr::FieldAccess(obj, field) => {
                self.compile_expr(obj);
                // Field access - simplified: just load the field as a call
                self.code
                    .push(Instruction::Call(field.clone(), 1));
            }
            Expr::ResultOk(inner) => {
                self.compile_expr(inner);
                self.code.push(Instruction::WrapOk);
            }
            Expr::ResultErr(inner) => {
                self.compile_expr(inner);
                self.code.push(Instruction::WrapErr);
            }
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

pub fn compile(func: &FuncDef) -> Vec<Instruction> {
    Compiler::new().compile_func(func)
}
