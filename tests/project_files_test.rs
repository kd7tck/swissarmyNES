#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use swissarmynes::server::project::{
        create_project, delete_file, list_files, read_file, write_file, PROJECTS_DIR,
    };

    fn cleanup(name: &str) {
        let path = Path::new(PROJECTS_DIR).join(name);
        if path.exists() {
            let _ = fs::remove_dir_all(path);
        }
    }

    #[test]
    fn test_file_operations() {
        let project_name = "test_file_ops";
        cleanup(project_name);
        create_project(project_name).unwrap();

        // 1. List files (should have main.swiss)
        let files = list_files(project_name).unwrap();
        assert!(files.contains(&"main.swiss".to_string()));
        // System files should be filtered out
        assert!(!files.contains(&"project.json".to_string()));

        // 2. Create new file
        let filename = "lib.swiss";
        let content = "SUB Lib()\nEND SUB";
        write_file(project_name, filename, content).unwrap();

        // 3. Read file
        let read_content = read_file(project_name, filename).unwrap();
        assert_eq!(read_content, content);

        // 4. List files again
        let files = list_files(project_name).unwrap();
        assert!(files.contains(&"main.swiss".to_string()));
        assert!(files.contains(&"lib.swiss".to_string()));

        // 5. Delete file
        delete_file(project_name, filename).unwrap();
        let files = list_files(project_name).unwrap();
        assert!(!files.contains(&"lib.swiss".to_string()));

        cleanup(project_name);
    }

    #[test]
    fn test_security() {
        let project_name = "test_security";
        cleanup(project_name);
        create_project(project_name).unwrap();

        // Path traversal
        assert!(read_file(project_name, "../outside.txt").is_err());
        assert!(write_file(project_name, "../outside.txt", "fail").is_err());

        // System files
        assert!(write_file(project_name, "project.json", "{}").is_err());
        assert!(delete_file(project_name, "project.json").is_err());
        assert!(delete_file(project_name, "main.swiss").is_err());

        cleanup(project_name);
    }
}
