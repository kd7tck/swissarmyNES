use crate::compiler::ast::DataType;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,   // Declared with DIM
    Constant,   // Declared with CONST
    Sub,        // SUB definition
    Param,      // SUB parameter
    Local,      // Local variable (implicit or explicit in FOR/LET)
    Struct,     // Struct definition
    Enum,       // Enum definition
    Metasprite, // Metasprite definition
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub data_type: DataType,
    pub kind: SymbolKind,
    pub address: Option<u16>,
    pub value: Option<i32>,            // Constants / Struct Size
    pub params: Option<Vec<DataType>>, // Subroutines
    pub members: Option<Vec<(String, DataType, u16)>>, // Struct members: Name, Type, Offset
    pub variants: Option<Vec<(String, i32)>>, // Enum variants: Name, Value
}

#[derive(Debug)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()], // Start with global scope
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define(
        &mut self,
        name: String,
        data_type: DataType,
        kind: SymbolKind,
    ) -> Result<(), String> {
        self.define_with_params(name, data_type, kind, None)
    }

    pub fn define_with_params(
        &mut self,
        name: String,
        data_type: DataType,
        kind: SymbolKind,
        params: Option<Vec<DataType>>,
    ) -> Result<(), String> {
        self.define_full(name, data_type, kind, params, None, None, None)
    }

    pub fn define_struct(
        &mut self,
        name: String,
        members: Vec<(String, DataType, u16)>,
        size: u16,
    ) -> Result<(), String> {
        self.define_full(
            name.clone(),
            DataType::Struct(name),
            SymbolKind::Struct,
            None,
            Some(members),
            Some(size as i32),
            None,
        )
    }

    pub fn define_enum(
        &mut self,
        name: String,
        variants: Vec<(String, i32)>,
    ) -> Result<(), String> {
        self.define_full(
            name.clone(),
            DataType::Enum(name),
            SymbolKind::Enum,
            None,
            None,
            None,
            Some(variants),
        )
    }

    pub fn define_metasprite(&mut self, name: String) -> Result<(), String> {
        self.define_full(
            name,
            DataType::Word, // Metasprite is referenced as a 16-bit address
            SymbolKind::Metasprite,
            None,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn define_full(
        &mut self,
        name: String,
        data_type: DataType,
        kind: SymbolKind,
        params: Option<Vec<DataType>>,
        members: Option<Vec<(String, DataType, u16)>>,
        value: Option<i32>,
        variants: Option<Vec<(String, i32)>>,
    ) -> Result<(), String> {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(&name) {
                return Err(format!(
                    "Symbol '{}' already defined in current scope",
                    name
                ));
            }
            let symbol = Symbol {
                name: name.clone(),
                data_type,
                kind,
                address: None,
                value,
                params,
                members,
                variants,
            };
            scope.insert(name, symbol);
            Ok(())
        } else {
            Err("No scope available".to_string())
        }
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        // Search from top (innermost) scope to bottom (global)
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }

    pub fn is_defined_locally(&self, name: &str) -> bool {
        if let Some(scope) = self.scopes.last() {
            scope.contains_key(name)
        } else {
            false
        }
    }

    pub fn assign_address(&mut self, name: &str, address: u16) -> Result<(), String> {
        // Search from top (innermost) scope to bottom (global)
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbol) = scope.get_mut(name) {
                symbol.address = Some(address);
                return Ok(());
            }
        }
        Err(format!("Symbol '{}' not found", name))
    }

    pub fn assign_value(&mut self, name: &str, value: i32) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbol) = scope.get_mut(name) {
                symbol.value = Some(value);
                return Ok(());
            }
        }
        Err(format!("Symbol '{}' not found", name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define_and_resolve_global() {
        let mut table = SymbolTable::new();
        table
            .define("x".to_string(), DataType::Byte, SymbolKind::Variable)
            .unwrap();

        let sym = table.resolve("x");
        assert!(sym.is_some());
        let sym = sym.unwrap();
        assert_eq!(sym.name, "x");
        assert_eq!(sym.data_type, DataType::Byte);
        assert_eq!(sym.kind, SymbolKind::Variable);
    }

    #[test]
    fn test_shadowing() {
        let mut table = SymbolTable::new();
        table
            .define("x".to_string(), DataType::Byte, SymbolKind::Variable)
            .unwrap();

        table.enter_scope();
        table
            .define("x".to_string(), DataType::Word, SymbolKind::Local)
            .unwrap();

        let sym = table.resolve("x");
        assert!(sym.is_some());
        let sym = sym.unwrap();
        assert_eq!(sym.data_type, DataType::Word);
        assert_eq!(sym.kind, SymbolKind::Local);

        table.exit_scope();

        let sym = table.resolve("x");
        assert!(sym.is_some());
        let sym = sym.unwrap();
        assert_eq!(sym.data_type, DataType::Byte);
        assert_eq!(sym.kind, SymbolKind::Variable);
    }

    #[test]
    fn test_duplicate_definition_error() {
        let mut table = SymbolTable::new();
        table
            .define("x".to_string(), DataType::Byte, SymbolKind::Variable)
            .unwrap();
        let res = table.define("x".to_string(), DataType::Byte, SymbolKind::Variable);
        assert!(res.is_err());
    }

    #[test]
    fn test_resolution_missing() {
        let table = SymbolTable::new();
        assert!(table.resolve("nonexistent").is_none());
    }
}
