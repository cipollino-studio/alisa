
use std::cell::RefCell;

use crate::{rmpv_get, Delta, KeyChain, LoadingContext, OperationDyn, Project, ProjectContext, Recorder, UnconfirmedOperation};

use super::{Client, ClientKind};


pub(crate) struct Collab<P: Project> {
    keychain: RefCell<KeyChain<3>>,
    key_request_sent: bool,

    unconfirmed_operations: Vec<UnconfirmedOperation<P>> 
}

impl<P: Project> Collab<P> {

    pub(crate) fn new() -> Self {
        Self {
            keychain: RefCell::new(KeyChain::new()),
            key_request_sent: false,
            unconfirmed_operations: Vec::new(),
        }
    }

    pub(crate) fn next_key(&self) -> Option<u64> {
        self.keychain.borrow_mut().next_key()
    }

    pub(crate) fn has_keys(&self) -> bool {
        self.keychain.borrow().has_keys()
    }

    pub(crate) fn accept_keys(&self, first: u64, last: u64) {
        self.keychain.borrow_mut().accept_keys(first, last);
    }

    pub(crate) fn request_keys(&mut self, messages: &mut Vec<rmpv::Value>) {
        let keychain = self.keychain.borrow_mut();
        if keychain.wants_keys() && !self.key_request_sent {
            messages.push(rmpv::Value::Map(vec![
                ("type".into(), "key_request".into())
            ]));
            self.key_request_sent = true;
        }
    }

    pub(crate) fn perform_operation(&mut self, operation: Box<dyn OperationDyn<Project = P>>, deltas: Vec<Box<dyn Delta<Project = P>>>, messages: &mut Vec<rmpv::Value>) {
        messages.push(rmpv::Value::Map(vec![
            ("type".into(), "operation".into()),
            ("operation".into(), operation.name().into()),
            ("data".into(), operation.serialize())
        ]));
        self.unconfirmed_operations.push(UnconfirmedOperation {
            operation,
            deltas
        });
    }

}

impl<P: Project> Client<P> {

    pub fn collab(welcome_data: rmpv::Value) -> Option<Self> {
        welcome_data.as_map()?;
        let project_data = rmpv_get(&welcome_data, "project")?;
        let mut objects = P::Objects::default();
        let project = P::load(project_data, &mut LoadingContext::collab(&mut objects))?;
        Some(Self {
            kind: ClientKind::Collab(Collab::new()),
            project,
            objects,
            operations_to_perform: RefCell::new(Vec::new()),
            project_modified: false
        })
    }

    pub(crate) fn handle_operation_message(&mut self, operation_name: &str, data: &rmpv::Value, context: &mut P::Context) -> Option<()> {
        // Find the type of operation being performed
        let operation_kind = P::OPERATIONS.iter().find(|kind| kind.name == operation_name)?;
        // Deserialize the operation from the message
        let operation = (operation_kind.deserialize)(data)?; 

        let mut project_context = ProjectContext {
            project: &mut self.project,
            objects: &mut self.objects,
            context,
            project_modified: &mut self.project_modified,
        };

        // Undo all the stuff we've done client side
        if let Some(collab) = self.kind.as_collab() {
            for unconfirmed_operation in collab.unconfirmed_operations.iter().rev() {
                for delta in unconfirmed_operation.deltas.iter().rev() {
                    delta.perform(&mut project_context);
                }
            }
        }

        // Apply the newly-received operation
        let mut recorder = Recorder::new(project_context);
        (operation_kind.perform)(operation, &mut recorder);

        // Reapply the operations we've done on top of the inserted operation
        if let Some(collab) = self.kind.as_collab() {
            for unconfirmed_operation in &collab.unconfirmed_operations {
                let mut recorder = Recorder::new(ProjectContext {
                    project: &mut self.project,
                    objects: &mut self.objects,
                    context,
                    project_modified: &mut self.project_modified,
                });
                unconfirmed_operation.operation.perform(&mut recorder);
            }
        }

        Some(())
    }

    pub fn receive_message(&mut self, msg: rmpv::Value, context: &mut P::Context) -> Option<()> {

        if !self.is_collab() {
            return None;
        }

        let msg = msg.as_map()?;
        let mut msg_type = "";
        let mut operation_name = "";
        let mut data = None;
        let mut first = 0;
        let mut last = 0;

        for (key, val) in msg {
            let key = key.as_str()?;
            match key {
                "type" => {
                    msg_type = val.as_str()?;
                },
                "operation" => {
                    operation_name = val.as_str()?;
                },
                "data" => {
                    data = Some(val);
                },
                "first" => {
                    first = val.as_u64()?;
                },
                "last" => {
                    last = val.as_u64()?;
                },
                _ => {}
            }
        }

        match msg_type {
            "confirm" => {
                if let Some(collab) = self.kind.as_collab() {
                    collab.unconfirmed_operations.remove(0);
                }
            },
            "operation" => {
                if let Some(data) = data {
                    self.handle_operation_message(operation_name, data, context);
                }
            },
            "key_grant" => {
                if first != 0 && last != 0 {
                    if let Some(collab) = self.kind.as_collab() {
                        collab.accept_keys(first, last);
                        collab.key_request_sent = false;
                    }
                }
            }
            _ => {}
        }

        Some(())
    }
   

}
