use crate::compiler::ast::{
    CaseCondition, Expression, Program, Statement, StatementKind, TopLevel, TopLevelKind,
};
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
        match &decl.kind {
            TopLevelKind::Include(filename) => {
                // Check if already included to prevent cycles and duplicates (Pragma Once behavior)
                if seen_files.contains(filename) {
                    continue;
                }
                seen_files.insert(filename.clone());

                let source = source_provider(filename)?;

                // Lex and Parse
                let mut lexer = Lexer::new(&source);
                let tokens = lexer
                    .tokenize()
                    .map_err(|e| format!("Lexer error in {}: {}", filename, e))?;

                let mut parser = Parser::new(tokens);
                let mut included_program = parser
                    .parse()
                    .map_err(|e| format!("Parser error in {}: {}", filename, e))?;
                for declaration in &mut included_program.declarations {
                    declaration.source_file = filename.clone();
                    match &mut declaration.kind {
                        TopLevelKind::Sub(_, _, body)
                        | TopLevelKind::Interrupt(_, body)
                        | TopLevelKind::Macro(_, _, body) => stamp_statements(body, filename, None),
                        _ => {}
                    }
                }

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
        match decl.kind {
            TopLevelKind::Macro(name, params, body) => {
                if macros.contains_key(&name) {
                    return Err(format!("Duplicate macro definition: {}", name));
                }
                macros.insert(name, MacroDef { params, body });
            }
            _ => new_declarations.push(decl),
        }
    }

    // 2. Expand macros in the remaining code
    let mut final_declarations = Vec::new();
    for decl in new_declarations {
        match decl.kind {
            TopLevelKind::Sub(name, params, body) => {
                let expanded_body = expand_statements(body, &macros, 0)?;
                final_declarations.push(TopLevel {
                    kind: TopLevelKind::Sub(name, params, expanded_body),
                    source_file: decl.source_file.clone(),
                    line: decl.line,
                });
            }
            TopLevelKind::Interrupt(name, body) => {
                let expanded_body = expand_statements(body, &macros, 0)?;
                final_declarations.push(TopLevel {
                    kind: TopLevelKind::Interrupt(name, expanded_body),
                    source_file: decl.source_file.clone(),
                    line: decl.line,
                });
            }
            // Declarations that don't contain statements:
            // Const, Dim, TypeDecl, Enum, Data, Asm, Include
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
        if let StatementKind::Call(Expression::Identifier(ref name), ref args) = stmt.kind {
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
                let mut fully_expanded = expand_statements(body_with_args, macros, depth + 1)?;
                // Expanded instructions belong to the call site for debugging.
                stamp_statements(&mut fully_expanded, &stmt.source_file, Some(stmt.line));

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
    match stmt.kind {
        StatementKind::If(cond, then_block, else_block) => Ok(Statement {
            kind: StatementKind::If(
                cond,
                expand_statements(then_block, macros, depth)?,
                if let Some(block) = else_block {
                    Some(expand_statements(block, macros, depth)?)
                } else {
                    None
                },
            ),
            source_file: stmt.source_file.clone(),
            line: stmt.line,
        }),
        StatementKind::While(cond, body) => Ok(Statement {
            kind: StatementKind::While(cond, expand_statements(body, macros, depth)?),
            source_file: stmt.source_file.clone(),
            line: stmt.line,
        }),
        StatementKind::DoWhile(body, cond) => Ok(Statement {
            kind: StatementKind::DoWhile(expand_statements(body, macros, depth)?, cond),
            source_file: stmt.source_file.clone(),
            line: stmt.line,
        }),
        StatementKind::For(var, start, end, step, body) => Ok(Statement {
            kind: StatementKind::For(
                var,
                start,
                end,
                step,
                expand_statements(body, macros, depth)?,
            ),
            source_file: stmt.source_file.clone(),
            line: stmt.line,
        }),
        StatementKind::Select(expr, cases, else_block) => {
            let mut new_cases = Vec::new();
            for (val, block) in cases {
                new_cases.push((val, expand_statements(block, macros, depth)?));
            }
            let new_else = if let Some(block) = else_block {
                Some(expand_statements(block, macros, depth)?)
            } else {
                None
            };
            Ok(Statement {
                kind: StatementKind::Select(expr, new_cases, new_else),
                source_file: stmt.source_file.clone(),
                line: stmt.line,
            })
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
    let new_kind = match &stmt.kind {
        StatementKind::Let(target, val) => StatementKind::Let(
            replace_args_in_expression(target.clone(), mapping),
            replace_args_in_expression(val.clone(), mapping),
        ),
        StatementKind::If(cond, then_b, else_b) => StatementKind::If(
            replace_args_in_expression(cond.clone(), mapping),
            replace_args_in_statements(then_b, mapping),
            else_b
                .as_ref()
                .map(|b| replace_args_in_statements(b, mapping)),
        ),
        StatementKind::While(cond, body) => StatementKind::While(
            replace_args_in_expression(cond.clone(), mapping),
            replace_args_in_statements(body, mapping),
        ),
        StatementKind::DoWhile(body, cond) => StatementKind::DoWhile(
            replace_args_in_statements(body, mapping),
            replace_args_in_expression(cond.clone(), mapping),
        ),
        StatementKind::For(var, start, end, step, body) => {
            let mut new_var = var.clone();
            if let Some(Expression::Identifier(v)) = mapping.get(var) {
                new_var = v.clone();
            }
            StatementKind::For(
                new_var,
                replace_args_in_expression(start.clone(), mapping),
                replace_args_in_expression(end.clone(), mapping),
                step.as_ref()
                    .map(|s| replace_args_in_expression(s.clone(), mapping)),
                replace_args_in_statements(body, mapping),
            )
        }
        StatementKind::Return(opt) => StatementKind::Return(
            opt.as_ref()
                .map(|e| replace_args_in_expression(e.clone(), mapping)),
        ),
        StatementKind::Call(target, args) => StatementKind::Call(
            replace_args_in_expression(target.clone(), mapping),
            args.iter()
                .map(|a| replace_args_in_expression(a.clone(), mapping))
                .collect(),
        ),
        StatementKind::Poke(addr, val) => StatementKind::Poke(
            replace_args_in_expression(addr.clone(), mapping),
            replace_args_in_expression(val.clone(), mapping),
        ),
        StatementKind::PlaySfx(id) => {
            StatementKind::PlaySfx(replace_args_in_expression(id.clone(), mapping))
        }
        StatementKind::Print(args) => StatementKind::Print(
            args.iter()
                .map(|a| replace_args_in_expression(a.clone(), mapping))
                .collect(),
        ),
        StatementKind::Select(expr, cases, else_b) => StatementKind::Select(
            replace_args_in_expression(expr.clone(), mapping),
            cases
                .iter()
                .map(|(conds, b)| {
                    (
                        conds
                            .iter()
                            .map(|cond| match cond {
                                CaseCondition::Value(e) => CaseCondition::Value(
                                    replace_args_in_expression(e.clone(), mapping),
                                ),
                                CaseCondition::Range(start, end) => CaseCondition::Range(
                                    replace_args_in_expression(start.clone(), mapping),
                                    replace_args_in_expression(end.clone(), mapping),
                                ),
                                CaseCondition::Is(op, e) => CaseCondition::Is(
                                    op.clone(),
                                    replace_args_in_expression(e.clone(), mapping),
                                ),
                            })
                            .collect(),
                        replace_args_in_statements(b, mapping),
                    )
                })
                .collect(),
            else_b
                .as_ref()
                .map(|b| replace_args_in_statements(b, mapping)),
        ),
        StatementKind::On(vec, sub) => {
            let mut new_vec = vec.clone();
            let mut new_sub = sub.clone();
            if let Some(Expression::Identifier(v)) = mapping.get(vec) {
                new_vec = v.clone();
            }
            if let Some(Expression::Identifier(s)) = mapping.get(sub) {
                new_sub = s.clone();
            }
            StatementKind::On(new_vec, new_sub)
        }
        _ => stmt.kind.clone(),
    };

    Statement {
        kind: new_kind,
        source_file: stmt.source_file.clone(),
        line: stmt.line,
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

// Preserve provenance recursively through includes and macro expansion.
fn stamp_statements(statements: &mut [Statement], file: &str, line: Option<usize>) {
    for statement in statements {
        statement.source_file = file.to_string();
        if let Some(line) = line {
            statement.line = line;
        }
        match &mut statement.kind {
            StatementKind::If(_, body, otherwise) => {
                stamp_statements(body, file, line);
                if let Some(body) = otherwise {
                    stamp_statements(body, file, line);
                }
            }
            StatementKind::While(_, body)
            | StatementKind::DoWhile(body, _)
            | StatementKind::For(_, _, _, _, body) => stamp_statements(body, file, line),
            StatementKind::Select(_, cases, otherwise) => {
                for (_, body) in cases {
                    stamp_statements(body, file, line);
                }
                if let Some(body) = otherwise {
                    stamp_statements(body, file, line);
                }
            }
            _ => {}
        }
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
                TopLevel {
                    kind: TopLevelKind::Include("lib.swiss".to_string()),
                    source_file: "main.swiss".to_string(),
                    line: 1,
                },
                TopLevel {
                    kind: TopLevelKind::Sub("Main".to_string(), vec![], vec![]),
                    source_file: "main.swiss".to_string(),
                    line: 2,
                },
            ],
        };

        let result = process_includes(program, &provider).expect("Failed to process includes");

        assert_eq!(result.declarations.len(), 2);
        // First should be LibSub
        if let TopLevelKind::Sub(name, _, _) = &result.declarations[0].kind {
            assert_eq!(name, "LibSub");
        } else {
            panic!("Expected LibSub");
        }
        // Second should be Main
        if let TopLevelKind::Sub(name, _, _) = &result.declarations[1].kind {
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
            declarations: vec![TopLevel {
                kind: TopLevelKind::Include("A.swiss".to_string()),
                source_file: "main.swiss".to_string(),
                line: 1,
            }],
        };

        let result = process_includes(program, &provider).expect("Failed to process includes");

        // Should contain SubB and SubA. Cycle should be broken.
        // trace: Include A -> (Include B -> (Include A -> Skip) + SubB) + SubA
        // Result: SubB, SubA

        assert_eq!(result.declarations.len(), 2);
        // Order: B then A (because A includes B first)
        if let TopLevelKind::Sub(name, _, _) = &result.declarations[0].kind {
            assert_eq!(name, "SubB");
        }
        if let TopLevelKind::Sub(name, _, _) = &result.declarations[1].kind {
            assert_eq!(name, "SubA");
        }
    }
}
