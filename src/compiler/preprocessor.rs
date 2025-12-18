use crate::compiler::ast::{Program, TopLevel};
use crate::compiler::lexer::Lexer;
use crate::compiler::parser::Parser;
use std::collections::HashSet;

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
