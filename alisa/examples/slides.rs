use alisa::Children;


#[derive(alisa::Serializable)]
#[project(SlipsProject)]
pub struct SlipsProject {
    name: String,
    slides: alisa::ChildList<Slide>
}

impl Default for SlipsProject {

    fn default() -> Self {
        Self {
            name: "Untitled Slips".to_string(),
            slides: alisa::ChildList::default()
        }
    }

}

impl alisa::Project for SlipsProject {

    type Context = ();
    type Objects = SlipsObjects;

    fn empty() -> Self {
        Self::default()
    }

    fn create_default(&mut self) {

    }

    const OBJECTS: &'static [alisa::ObjectKind<Self>] = &[
        alisa::ObjectKind::from::<Slide>()
    ];

    const OPERATIONS: &'static [alisa::OperationKind<Self>] = &[
        alisa::OperationKind::from::<SetName>(),
        alisa::OperationKind::from::<CreateSlide>()
    ];

}

alisa::project_set_property_operation!(SlipsProject, name, String);

#[derive(alisa::Serializable, Clone)]
#[project(SlipsProject)]
pub struct Slide {
    parent: (),
    title: String,
}

impl Default for Slide {

    fn default() -> Self {
        Self {
            parent: (),
            title: "Top Text".to_owned(),
        }
    }

}

impl alisa::Object for Slide {
    type Project = SlipsProject;

    const NAME: &'static str = "Slide";

    fn list(objects: &SlipsObjects) -> &alisa::ObjList<Slide> {
        &objects.slides
    }

    fn list_mut(objects: &mut SlipsObjects) -> &mut alisa::ObjList<Slide> {
        &mut objects.slides
    }
}

alisa::object_set_property_operation!(Slide, title, String);

#[derive(alisa::Serializable)]
#[project(SlipsProject)]
pub struct SlideTreeData {
    title: String
}

impl Default for SlideTreeData {

    fn default() -> Self {
        Self {
            title: "Slide".to_owned()
        }
    }

}

impl alisa::TreeObj for Slide {

    type ParentPtr = ();
    type ChildList = alisa::ChildList<Slide>;
    type TreeData = SlideTreeData;

    fn child_list<'a>(parent: (), project: &'a SlipsProject, objects: &'a SlipsObjects) -> Option<&'a alisa::ChildList<Slide>> {
        Some(&project.slides)
    }

    fn child_list_mut<'a>(parent: Self::ParentPtr, context: &'a mut alisa::ProjectContext<Self::Project>) -> Option<&'a mut Self::ChildList> {
        Some(&mut context.project_mut().slides)
    }

    fn parent(&self) -> () {
        self.parent
    }

    fn parent_mut(&mut self) -> &mut () {
        &mut self.parent
    }

    fn instance(data: &SlideTreeData, ptr: alisa::Ptr<Slide>, parent: (), recorder: &mut alisa::Recorder<SlipsProject>) {
        use alisa::Object;
        Self::add(recorder, ptr, Slide {
            parent,
            title: data.title.clone()
        });
    }

    fn destroy(&self, recorder: &mut alisa::Recorder<SlipsProject>) {
        
    }

    fn collect_data(&self, objects: &<Self::Project as alisa::Project>::Objects) -> Self::TreeData {
        SlideTreeData {
            title: self.title.clone(),
        }
    }

}

alisa::tree_object_creation_operations!(Slide);

pub struct SlipsObjects {
    slides: alisa::ObjList<Slide>
}

impl Default for SlipsObjects {

    fn default() -> Self {
        Self {
            slides: alisa::ObjList::default()
        }
    }

}

fn main() {

    let mut client = alisa::Client::<SlipsProject>::local("my_cool_path.slips").unwrap();

    let mut action = alisa::Action::new();
    client.perform(&mut action, SetName {
        name: "My Cool Name".to_string(),
    });

    if let Some(ptr) = client.next_ptr() {
        client.perform(&mut action, CreateSlide {
            ptr,
            parent: (),
            idx: client.project().slides.n_children(),
            data: SlideTreeData {
                title: "New Slide".to_owned(),
            },
        });
    }

    client.tick(&mut ());

    for slide_ptr in client.project().slides.iter() {
        if let Some(slide) = client.get(slide_ptr) {
            println!("{}", slide.title);
        }
    }
    
    let mut undo_redo = alisa::UndoRedoManager::new();

    // Add the action to the list of undo's 
    undo_redo.add(action);

    // If there's an action to undo, undo it
    undo_redo.undo(&client);

    client.tick(&mut ());

}
