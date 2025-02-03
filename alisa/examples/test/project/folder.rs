

use alisa::Object;

use super::{Project, ProjectObjects, SetFoldersDelta};

#[derive(Clone, alisa::Loadable)]
#[project(Project)]
pub struct Folder {
    pub name: String,
    pub myself: alisa::ObjBox<Folder>
}

impl Default for Folder {

    fn default() -> Self {
        Self { name: "Folder".to_owned(), myself: alisa::ObjBox::new(alisa::ObjPtr::null()) }
    }

}

impl alisa::Object for Folder {

    type Project = Project;

    const NAME: &'static str = "Folder";

    fn list(objects: &ProjectObjects) -> &alisa::ObjList<Self> {
        &objects.folders
    }

    fn list_mut(objects: &mut ProjectObjects) -> &mut alisa::ObjList<Self> {
        &mut objects.folders
    }

}

#[derive(alisa::Serializable)]
pub struct CreateFolder {
    pub ptr: alisa::ObjPtr<Folder>,
    pub name: String
}

impl Default for CreateFolder {

    fn default() -> Self {
        Self { ptr: alisa::ObjPtr::null(), name: "Folder".to_string() }
    }

}

#[derive(alisa::Serializable)]
pub struct DeleteFolder {
    pub ptr: alisa::ObjPtr<Folder>
}

impl Default for DeleteFolder {

    fn default() -> Self {
        Self { ptr: alisa::ObjPtr::null() }
    }

}

impl alisa::Operation for CreateFolder {

    type Project = Project;

    type Inverse = DeleteFolder;

    const NAME: &'static str = "CreateFolder";

    fn perform(&self, recorder: &mut alisa::Recorder<'_, Self::Project>) {
        recorder.push_delta(SetFoldersDelta {
            folders: recorder.project().folders.clone() 
        });
        recorder.project_mut().folders.push(alisa::ObjBox::new(self.ptr)); 
        Folder::add(recorder, self.ptr, Folder {
            name: self.name.clone(),
            myself: alisa::ObjBox::new(self.ptr),
        });
    }

    fn inverse(&self, _project: &Self::Project, _objects: &ProjectObjects) -> Option<Self::Inverse> {
        Some(DeleteFolder {
            ptr: self.ptr,
        })
    }

}

impl alisa::Operation for DeleteFolder {
    type Project = Project;

    type Inverse = CreateFolder;

    const NAME: &'static str = "DeleteFolder";

    fn perform(&self, recorder: &mut alisa::Recorder<'_, Self::Project>) {
        recorder.push_delta(SetFoldersDelta {
            folders: recorder.project().folders.clone() 
        });
        recorder.project_mut().folders.retain(|other| other.ptr() != self.ptr);
        Folder::delete(recorder, self.ptr);
    }

    fn inverse(&self, _project: &Self::Project, objects: &ProjectObjects) -> Option<Self::Inverse> {
        Some(CreateFolder {
            ptr: self.ptr,
            name: objects.folders.get(self.ptr).map(|folder| folder.name.clone()).unwrap_or("Folder".to_owned()),
        })
    }
}

alisa::object_set_property_operation!(Folder, name, String);
