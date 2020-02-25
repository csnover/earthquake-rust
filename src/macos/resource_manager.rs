struct Manager<'a, T: Reader> {
    current_file: Option<&'a ResourceFile<T>>,
    files: Vec<ResourceFile<T>>,
}

impl<'a, T: Reader> Manager<'a, T> {
    pub fn new() -> Self {
        todo!();
    }

    pub fn add_resource(resource: Resource) -> AResult<()> {
        todo!();
    }

    pub fn close_resource_file(index: usize) -> AResult<()> {
        todo!();
    }

    pub fn count_one_resources(kind: OSType) -> u32 {
        todo!();
    }

    pub fn count_resources(kind: OSType) -> u32 {
        todo!();
    }

    pub fn get_string(id: i16) -> Option<String> {
        todo!();
    }

    pub fn get_indexed_string(id: i16, index: i16) -> Option<String> {
        todo!();
    }

    pub fn get_indexed_resource(index: i16) -> Option<Resource> {
        todo!();
    }

    pub fn get_named_resource(name: &str) -> Option<Resource> {
        todo!();
    }

    pub fn get_one_indexed_resource(index: i16) -> Option<Resource> {
        todo!();
    }

    pub fn get_one_named_resource(name: &str) -> Option<Resource> {
        todo!();
    }

    pub fn get_one_resource(id: ResourceId) -> Option<Resource> {
        todo!();
    }

    pub fn get_resource(id: ResourceId) -> Option<Resource> {
        todo!();
    }

    pub fn open_resource_file(filename: &str) -> AResult<()> {
        todo!();
    }
}
