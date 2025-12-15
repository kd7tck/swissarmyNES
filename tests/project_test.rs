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
    fn test_attribute_persistence() {
        use swissarmynes::server::project::Nametable;

        let name = "test_project_attrs";
        cleanup(name);
        create_project(name).unwrap();

        let project = get_project(name).unwrap();
        let mut assets = project.assets.unwrap();

        // Add a nametable with specific attributes
        let mut attrs = vec![0; 64];
        attrs[0] = 0xFF; // Set first byte to all 1s (all palette 3)

        assets.nametables.push(Nametable {
            name: "NT1".to_string(),
            data: vec![0; 960],
            attrs: attrs.clone(),
        });

        assert!(save_project(name, None, Some(&assets)).is_ok());

        // Reload
        let project = get_project(name).unwrap();
        let loaded_assets = project.assets.unwrap();
        let loaded_nt = &loaded_assets.nametables[0];

        assert_eq!(loaded_nt.attrs.len(), 64);
        assert_eq!(loaded_nt.attrs[0], 0xFF);

        cleanup(name);
    }

    #[test]
    fn test_save_project() {
        let name = "test_project_2";
        cleanup(name);
        create_project(name).unwrap();

        let new_source = "CONST FOO = 1";
        assert!(save_project(name, Some(new_source), None).is_ok());

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
