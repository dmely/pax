use pax_core::{
    with_properties_unwrapped, ExpandedNode, HandlerRegistry, InstanceNode, InstanceNodePtr,
    InstanceNodePtrList, InstantiationArgs, PropertiesTreeContext,
};
use pax_std::primitives::{Group, Rectangle};
use piet_common::RenderContext;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use pax_runtime_api::{CommonProperties, Layer};

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
pub struct GroupInstance<R: 'static + RenderContext> {
    pub instance_id: u32,
    pub instance_children: InstanceNodePtrList<R>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,

    instance_prototypical_properties_factory: Box<dyn Fn() -> Rc<RefCell<dyn Any>>>,
    instance_prototypical_common_properties_factory: Box<dyn Fn() -> Rc<RefCell<CommonProperties>>>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for GroupInstance<R> {
    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }

    fn get_instance_children(&self) -> InstanceNodePtrList<R> {
        Rc::clone(&self.instance_children)
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        let mut node_registry = args.node_registry.borrow_mut();
        let instance_id = node_registry.mint_instance_id();
        let ret = Rc::new(RefCell::new(Self {
            instance_id,
            instance_children: match args.children {
                None => Rc::new(RefCell::new(vec![])),
                Some(children) => children,
            },
            handler_registry: args.handler_registry,

            instance_prototypical_common_properties_factory: args
                .prototypical_common_properties_factory,
            instance_prototypical_properties_factory: args.prototypical_properties_factory,
        }));

        node_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }

    fn expand(&self, ptc: &mut PropertiesTreeContext<R>) -> Rc<RefCell<ExpandedNode<R>>> {
        ExpandedNode::get_or_create_with_prototypical_properties(
            self.instance_id,
            ptc,
            &(self.instance_prototypical_properties_factory)(),
            &(self.instance_prototypical_common_properties_factory)(),
        )
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry>>> {
        match &self.handler_registry {
            Some(registry) => Some(Rc::clone(&registry)),
            _ => None,
        }
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }

    fn is_invisible_to_raycasting(&self) -> bool {
        true
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode<R>>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => {
                with_properties_unwrapped!(
                    &expanded_node.get_properties(),
                    Group,
                    |r: &mut Group| { f.debug_struct("Group").finish() }
                )
            }
            None => f.debug_struct("Group").finish_non_exhaustive(),
        }
    }
}
