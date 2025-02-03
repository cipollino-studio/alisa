
use std::{cell::RefCell, marker::PhantomData, path::Path};

use crate::{rmpv_encode, rmpv_get, LoadingContext, Project, StoringContext};

use super::{Client, ClientKind};

pub(crate) struct Local<P: Project> {
    /// The Verter file to which the project is saved
    file: verter::File,
    /// The next key available for use
    curr_key: RefCell<u64>,
    /// The pointer to the project data in the Verter file
    project_ptr: u64,
    /// Does the root data of the Verter file need to be updated?
    root_data_modified: RefCell<bool>,

    /// Marker to make sure the type `P`` is used
    _marker: PhantomData<P>
}

impl<P: Project> Local<P> {

    pub(crate) fn new(file: verter::File, curr_key: u64, project_ptr: u64) -> Self {
        Self {
            file,
            curr_key: RefCell::new(curr_key),
            project_ptr,
            root_data_modified: RefCell::new(false),
            _marker: PhantomData
        }
    }

    pub(crate) fn next_key(&self) -> u64 {
        let mut curr_key = self.curr_key.borrow_mut();
        let key = *curr_key;
        *curr_key += 1;
        *self.root_data_modified.borrow_mut() = true;
        key
    }

    pub(crate) fn next_key_range(&self, n_keys: u64) -> (u64, u64) {
        let mut curr_key = self.curr_key.borrow_mut();
        let first = *curr_key;
        *curr_key += n_keys;
        *self.root_data_modified.borrow_mut() = true;
        (first, first + n_keys - 1)
    }

    pub(crate) fn file(&mut self) -> &mut verter::File {
        &mut self.file
    }

    fn update_root_data(&mut self) {
        let root_data = rmpv::Value::Map(vec![
            ("curr_key".into(), (*self.curr_key.borrow()).into()),
            ("proj_ptr".into(), self.project_ptr.into())
        ]);
        if let Some(root_data) = rmpv_encode(&root_data) {
            let _ = self.file.write_root(&root_data);
        }
    }

    pub(crate) fn save_changes(&mut self, project: &mut P, objects: &mut P::Objects, project_modified: &mut bool) {

        // Update file root data if necessary 
        if *self.root_data_modified.borrow() {
            self.update_root_data();
            *self.root_data_modified.borrow_mut() = false;
        }

        // Project modifications
        if *project_modified {
            let data = project.store(&StoringContext::shallow(objects, &mut self.file));
            if let Some(data) = rmpv_encode(&data) {
                let _ = self.file.write(self.project_ptr, &data);
            }
            *project_modified = false;
        }

        // Object modifications
        for object_kind in P::OBJECTS {
            (object_kind.save_modifications)(&mut self.file, objects);
        }

    }

}

impl<P: Project> Client<P> {

    pub fn local<PathRef: AsRef<Path>>(path: PathRef) -> Option<Self> {
        let mut file = verter::File::open(path, verter::Config::default()).ok()?; // TODO: add configuration for magic bytes

        // Load important file metadata
        let root_data = file.read_root().ok()?;
        let mut root_data = root_data.as_slice();
        let root_data = rmpv::decode::read_value(&mut root_data).ok();
        let curr_key = root_data.as_ref().map(|data| rmpv_get(data, "curr_key")).flatten().map(|key| key.as_u64()).flatten();
        let proj_ptr = root_data.as_ref().map(|data| rmpv_get(data, "proj_ptr")).flatten().map(|ptr| ptr.as_u64()).flatten();

        // Load the root project
        let proj_data = proj_ptr.map(|ptr| file.read(ptr).ok()).flatten().map(|data| rmpv::decode::read_value(&mut data.as_slice()).ok()).flatten();
        let mut objects = P::Objects::default();
        let mut loading_context = LoadingContext::local(&mut objects, &mut file);
        let project = proj_data.map(|data| P::load(&data, &mut loading_context)).flatten();

        let (project, local) = if curr_key.is_some() && project.is_some() && proj_ptr.is_some() {
            // The project already exists! Yay! Nothing to see here...
            (project.unwrap(), Local::new(file, curr_key.unwrap(), proj_ptr.unwrap()))
        } else {
            // We need to create the project
            let mut project = P::empty();
            project.create_default();

            // Store the newly-created project
            let proj_data_ptr = file.alloc().ok()?;
            let storing_context = StoringContext::shallow(&objects, &mut file);
            if let Some(proj_data) = rmpv_encode(&project.store(&storing_context)) {
                file.write(proj_data_ptr, &proj_data).ok()?;
            }

            let curr_key = curr_key.unwrap_or(1);
            let mut local = Local::new(file, curr_key, proj_data_ptr);
            local.update_root_data();

            (project, local) 
        };

        Some(Self {
            kind: ClientKind::Local(local),
            project,
            objects,
            operations_to_perform: RefCell::new(Vec::new()),
            project_modified: false
        })
    }

}
