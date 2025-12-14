#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use swissarmynes::server::project::{create_project, get_project, list_projects, save_project};

    const TEST_PROJECTS_DIR: &str = "projects";

    fn cleanup(name: &str) {
        let path = Path::new(TEST_PROJECTS_DIR).join(name);
        if path.exists() {
            fs::remove_dir_all(path).unwrap();
        }
    }

    #[test]
    fn test_create_and_get_project() {
        let name = "test_project_1";
        cleanup(name);

        assert!(create_project(name).is_ok());

        let projects = list_projects().unwrap();
        assert!(projects.contains(&name.to_string()));

        let project = get_project(name).unwrap();
        assert_eq!(project.metadata.name, name);
        assert!(project.source.contains("SUB Main()"));

        cleanup(name);
    }

    #[test]
    fn test_save_project() {
        let name = "test_project_2";
        cleanup(name);
        create_project(name).unwrap();

        let new_source = "CONST FOO = 1";
        assert!(save_project(name, new_source).is_ok());

        let project = get_project(name).unwrap();
        assert_eq!(project.source, new_source);

        cleanup(name);
    }

    #[test]
    fn test_invalid_names() {
        assert!(create_project("invalid name").is_err());
        assert!(create_project("test/../test").is_err());
    }
}
