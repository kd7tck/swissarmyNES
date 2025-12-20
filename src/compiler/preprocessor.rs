use crate::compiler::ast::{Expression, Program, Statement, TopLevel};
use crate::compiler::lexer::Lexer;
use crate::compiler::parser::Parser;
use std::collections::{HashMap, HashSet};

pub fn process_includes(
    program: Program,
    source_provider: &dyn Fn(&str) -> Result<String, String>,
) -> Result<Program, String> {
    let mut seen_files = HashSet::new();
    expand_program(program, source_provider, &mut seen_files)
}

fn expand_program(
    program: Program,
    source_provider: &dyn Fn(&str) -> Result<String, String>,
    seen_files: &mut HashSet<String>,
) -> Result<Program, String> {
    let mut new_declarations = Vec::new();

    for decl in program.declarations {
        match decl {
            TopLevel::Include(filename) => {
                // Check if already included to prevent cycles and duplicates (Pragma Once behavior)
                if seen_files.contains(&filename) {
                    // Start of circular dependency check vs duplicate inclusion check.
                    // If we want to allow including the same file if it's not a cycle but just shared dependency (diamond problem),
                    // we usually still only want to include it ONCE to avoid symbol redefinition.
                    // So skipping it is the correct behavior for "Pragma Once".
                    // If it was a cycle, we'd still skip it, breaking the cycle.
                    continue;
                }
                seen_files.insert(filename.clone());

                let source = source_provider(&filename)?;

                // Lex and Parse
                let mut lexer = Lexer::new(&source);
                let tokens = lexer
                    .tokenize()
                    .map_err(|e| format!("Lexer error in {}: {}", filename, e))?;

                let mut parser = Parser::new(tokens);
                let included_program = parser
                    .parse()
                    .map_err(|e| format!("Parser error in {}: {}", filename, e))?;

                // Recursively expand
                let expanded_program =
                    expand_program(included_program, source_provider, seen_files)?;

                new_declarations.extend(expanded_program.declarations);
            }
            _ => new_declarations.push(decl),
        }
    }

    Ok(Program {
        declarations: new_declarations,
    })
}

struct MacroDef {
    params: Vec<String>,
    body: Vec<Statement>,
}

pub fn expand_macros(program: Program) -> Result<Program, String> {
    let mut macros = HashMap::new();
    let mut new_declarations = Vec::new();

    // 1. Collect Macros and filter them out
    for decl in program.declarations {
        if let TopLevel::Macro(name, params, body) = decl {
            if macros.contains_key(&name) {
                return Err(format!("Duplicate macro definition: {}", name));
            }
            macros.insert(name, MacroDef { params, body });
        } else {
            new_declarations.push(decl);
        }
    }

    // 2. Expand macros in the remaining code
    let mut final_declarations = Vec::new();
    for decl in new_declarations {
        match decl {
            TopLevel::Sub(name, params, body) => {
                let expanded_body = expand_statements(body, &macros, 0)?;
                final_declarations.push(TopLevel::Sub(name, params, expanded_body));
            }
            TopLevel::Interrupt(name, body) => {
                let expanded_body = expand_statements(body, &macros, 0)?;
                final_declarations.push(TopLevel::Interrupt(name, expanded_body));
            }
            // Declarations that don't contain statements:
            // Const, Dim, TypeDecl, Enum, Data, Asm, Include
            // (Include should be processed already)
            _ => final_declarations.push(decl),
        }
    }

    Ok(Program {
        declarations: final_declarations,
    })
}

fn expand_statements(
    stmts: Vec<Statement>,
    macros: &HashMap<String, MacroDef>,
    depth: usize,
) -> Result<Vec<Statement>, String> {
    if depth > 100 {
        return Err("Macro expansion recursion limit exceeded".to_string());
    }

    let mut new_stmts = Vec::new();
    for stmt in stmts {
        // Check for Call statement that matches a macro
        let mut expanded = false;
        if let Statement::Call(Expression::Identifier(ref name), ref args) = stmt {
            if let Some(macro_def) = macros.get(name) {
                // It is a macro call!
                if args.len() != macro_def.params.len() {
                    return Err(format!(
                        "Macro {} expects {} arguments, got {}",
                        name,
                        macro_def.params.len(),
                        args.len()
                    ));
                }

                // Map parameters to arguments
                let mut mapping = HashMap::new();
                for (i, param) in macro_def.params.iter().enumerate() {
                    mapping.insert(param.clone(), args[i].clone());
                }

                // 1. Replace arguments in the macro body
                let body_with_args = replace_args_in_statements(&macro_def.body, &mapping);

                // 2. Recursively expand any macros inside the result
                let fully_expanded = expand_statements(body_with_args, macros, depth + 1)?;

                new_stmts.extend(fully_expanded);
                expanded = true;
            }
        }

        if !expanded {
            // Not a macro call (or not a call at all), but might contain nested statements
            // We need to traverse down
            new_stmts.push(expand_nested_statements(stmt, macros, depth)?);
        }
    }
    Ok(new_stmts)
}

fn expand_nested_statements(
    stmt: Statement,
    macros: &HashMap<String, MacroDef>,
    depth: usize,
) -> Result<Statement, String> {
    match stmt {
        Statement::If(cond, then_block, else_block) => Ok(Statement::If(
            cond,
            expand_statements(then_block, macros, depth)?,
            if let Some(block) = else_block {
                Some(expand_statements(block, macros, depth)?)
            } else {
                None
            },
        )),
        Statement::While(cond, body) => Ok(Statement::While(
            cond,
            expand_statements(body, macros, depth)?,
        )),
        Statement::DoWhile(body, cond) => Ok(Statement::DoWhile(
            expand_statements(body, macros, depth)?,
            cond,
        )),
        Statement::For(var, start, end, step, body) => Ok(Statement::For(
            var,
            start,
            end,
            step,
            expand_statements(body, macros, depth)?,
        )),
        Statement::Select(expr, cases, else_block) => {
            let mut new_cases = Vec::new();
            for (val, block) in cases {
                new_cases.push((val, expand_statements(block, macros, depth)?));
            }
            let new_else = if let Some(block) = else_block {
                Some(expand_statements(block, macros, depth)?)
            } else {
                None
            };
            Ok(Statement::Select(expr, new_cases, new_else))
        }
        _ => Ok(stmt),
    }
}

fn replace_args_in_statements(
    stmts: &[Statement],
    mapping: &HashMap<String, Expression>,
) -> Vec<Statement> {
    stmts
        .iter()
        .map(|stmt| replace_args_in_statement(stmt, mapping))
        .collect()
}

fn replace_args_in_statement(stmt: &Statement, mapping: &HashMap<String, Expression>) -> Statement {
    match stmt {
        Statement::Let(target, val) => Statement::Let(
            replace_args_in_expression(target.clone(), mapping),
            replace_args_in_expression(val.clone(), mapping),
        ),
        Statement::If(cond, then_b, else_b) => Statement::If(
            replace_args_in_expression(cond.clone(), mapping),
            replace_args_in_statements(then_b, mapping),
            else_b
                .as_ref()
                .map(|b| replace_args_in_statements(b, mapping)),
        ),
        Statement::While(cond, body) => Statement::While(
            replace_args_in_expression(cond.clone(), mapping),
            replace_args_in_statements(body, mapping),
        ),
        Statement::DoWhile(body, cond) => Statement::DoWhile(
            replace_args_in_statements(body, mapping),
            replace_args_in_expression(cond.clone(), mapping),
        ),
        Statement::For(var, start, end, step, body) => {
            let mut new_var = var.clone();
            if let Some(Expression::Identifier(v)) = mapping.get(var) {
                new_var = v.clone();
            }
            Statement::For(
                new_var,
                replace_args_in_expression(start.clone(), mapping),
                replace_args_in_expression(end.clone(), mapping),
                step.as_ref()
                    .map(|s| replace_args_in_expression(s.clone(), mapping)),
                replace_args_in_statements(body, mapping),
            )
        }
        Statement::Return(opt) => Statement::Return(
            opt.as_ref()
                .map(|e| replace_args_in_expression(e.clone(), mapping)),
        ),
        Statement::Call(target, args) => Statement::Call(
            replace_args_in_expression(target.clone(), mapping),
            args.iter()
                .map(|a| replace_args_in_expression(a.clone(), mapping))
                .collect(),
        ),
        Statement::Poke(addr, val) => Statement::Poke(
            replace_args_in_expression(addr.clone(), mapping),
            replace_args_in_expression(val.clone(), mapping),
        ),
        Statement::PlaySfx(id) => {
            Statement::PlaySfx(replace_args_in_expression(id.clone(), mapping))
        }
        Statement::Print(args) => Statement::Print(
            args.iter()
                .map(|a| replace_args_in_expression(a.clone(), mapping))
                .collect(),
        ),
        Statement::Select(expr, cases, else_b) => Statement::Select(
            replace_args_in_expression(expr.clone(), mapping),
            cases
                .iter()
                .map(|(e, b)| {
                    (
                        replace_args_in_expression(e.clone(), mapping),
                        replace_args_in_statements(b, mapping),
                    )
                })
                .collect(),
            else_b
                .as_ref()
                .map(|b| replace_args_in_statements(b, mapping)),
        ),
        Statement::On(vec, sub) => {
            let mut new_vec = vec.clone();
            let mut new_sub = sub.clone();
            if let Some(Expression::Identifier(v)) = mapping.get(vec) {
                new_vec = v.clone();
            }
            if let Some(Expression::Identifier(s)) = mapping.get(sub) {
                new_sub = s.clone();
            }
            Statement::On(new_vec, new_sub)
        }
        _ => stmt.clone(),
    }
}

fn replace_args_in_expression(
    expr: Expression,
    mapping: &HashMap<String, Expression>,
) -> Expression {
    match expr {
        Expression::Identifier(ref name) => {
            if let Some(replacement) = mapping.get(name) {
                replacement.clone()
            } else {
                expr
            }
        }
        Expression::BinaryOp(l, op, r) => Expression::BinaryOp(
            Box::new(replace_args_in_expression(*l, mapping)),
            op,
            Box::new(replace_args_in_expression(*r, mapping)),
        ),
        Expression::UnaryOp(op, inner) => {
            Expression::UnaryOp(op, Box::new(replace_args_in_expression(*inner, mapping)))
        }
        Expression::Call(target, args) => Expression::Call(
            Box::new(replace_args_in_expression(*target, mapping)),
            args.iter()
                .map(|a| replace_args_in_expression(a.clone(), mapping))
                .collect(),
        ),
        Expression::Peek(inner) => {
            Expression::Peek(Box::new(replace_args_in_expression(*inner, mapping)))
        }
        Expression::MemberAccess(inner, member) => Expression::MemberAccess(
            Box::new(replace_args_in_expression(*inner, mapping)),
            member,
        ),
        _ => expr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_process_includes() {
        // Mock source provider
        let mut sources = HashMap::new();
        sources.insert("lib.swiss".to_string(), "SUB LibSub()\nEND SUB".to_string());

        let provider = |name: &str| {
            sources
                .get(name)
                .cloned()
                .ok_or(format!("File not found: {}", name))
        };

        // Main program with INCLUDE
        let program = Program {
            declarations: vec![
                TopLevel::Include("lib.swiss".to_string()),
                TopLevel::Sub("Main".to_string(), vec![], vec![]),
            ],
        };

        let result = process_includes(program, &provider).expect("Failed to process includes");

        assert_eq!(result.declarations.len(), 2);
        // First should be LibSub
        if let TopLevel::Sub(name, _, _) = &result.declarations[0] {
            assert_eq!(name, "LibSub");
        } else {
            panic!("Expected LibSub");
        }
        // Second should be Main
        if let TopLevel::Sub(name, _, _) = &result.declarations[1] {
            assert_eq!(name, "Main");
        } else {
            panic!("Expected Main");
        }
    }

    #[test]
    fn test_circular_include_prevention() {
        // A includes B, B includes A
        let mut sources = HashMap::new();
        sources.insert(
            "A.swiss".to_string(),
            "INCLUDE \"B.swiss\"\nSUB SubA()\nEND SUB".to_string(),
        );
        sources.insert(
            "B.swiss".to_string(),
            "INCLUDE \"A.swiss\"\nSUB SubB()\nEND SUB".to_string(),
        );

        let provider = |name: &str| {
            sources
                .get(name)
                .cloned()
                .ok_or(format!("File not found: {}", name))
        };

        // Start with A
        let program = Program {
            declarations: vec![TopLevel::Include("A.swiss".to_string())],
        };

        let result = process_includes(program, &provider).expect("Failed to process includes");

        // Should contain SubB and SubA. Cycle should be broken.
        // trace: Include A -> (Include B -> (Include A -> Skip) + SubB) + SubA
        // Result: SubB, SubA

        assert_eq!(result.declarations.len(), 2);
        // Order: B then A (because A includes B first)
        if let TopLevel::Sub(name, _, _) = &result.declarations[0] {
            assert_eq!(name, "SubB");
        }
        if let TopLevel::Sub(name, _, _) = &result.declarations[1] {
            assert_eq!(name, "SubA");
        }
    }
}
