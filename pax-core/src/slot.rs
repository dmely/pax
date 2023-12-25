use core::option::Option;

use std::rc::Rc;

use crate::{
    declarative_macros::handle_vtable_update, BaseInstance, ExpandedNode, InstanceFlags,
    InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_runtime_api::Layer;

/// A special "control-flow" primitive (a la `yield` or perhaps `goto`) — represents a slot into which
/// an slot_child can be rendered.  Slot relies on `slot_children` being present
/// on the [`Runtime`] stack and will not render any content if there are no `slot_children` found.
///
/// Consider a Stacker:  the owner of a Stacker passes the Stacker some nodes to render
/// inside the cells of the Stacker.  To the owner of the Stacker, those nodes might seem like
/// "children," but to the Stacker they are "slot_children" — children provided from
/// the outside.  Inside Stacker's template, there are a number of Slots — this primitive —
/// that become the final rendered home of those slot_children.  This same technique
/// is portable and applicable elsewhere via Slot.
pub struct SlotInstance {
    base: BaseInstance,
}

///Contains the index value for slot, either a literal or an expression.
#[derive(Default)]
pub struct SlotProperties {
    pub index: Box<dyn pax_runtime_api::PropertyInstance<pax_runtime_api::Numeric>>,
    last_index: usize,
    last_node_id: u32,
}

impl InstanceNode for SlotInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                    is_component: false,
                },
            ),
        })
    }

    fn recompute_children(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &mut RuntimeContext,
    ) {
        let node = expanded_node.with_properties_unwrapped(|properties: &mut SlotProperties| {
            let index: usize = properties
                .index
                .get()
                .get_as_int()
                .try_into()
                .expect("Slot index must be non-negative");
            expanded_node
                .containing_component
                .upgrade()
                .as_ref()
                .expect("slot has containing component during create")
                .expanded_and_flattened_slot_children
                .borrow()
                .as_ref()
                .and_then(|v| v.get(index))
                .map(|v| Rc::clone(&v))
        });

        if let Some(node) = node {
            expanded_node.attach_children(vec![Rc::clone(&node)], context);
        }
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
        let update_child =
            expanded_node.with_properties_unwrapped(|properties: &mut SlotProperties| {
                handle_vtable_update(
                    &context.expression_table(),
                    &expanded_node.stack,
                    &mut properties.index,
                );
                let curr_index: usize = properties.index.get().get_as_int().try_into().unwrap();
                // TODO get current node from the containing components list of slot children.
                let curr_node_id = expanded_node
                    .containing_component
                    .upgrade()
                    .expect("slot has containing component")
                    .expanded_and_flattened_slot_children
                    .borrow()
                    .as_ref()
                    .unwrap_or(&vec![])
                    .get(curr_index)
                    .map(|v| v.id_chain[0]);
                let update_child = properties.last_index != curr_index
                    || curr_node_id.is_some_and(|v| v != properties.last_node_id);
                properties.last_index = curr_index;
                properties.last_node_id = curr_node_id.unwrap_or(100000);
                update_child
            });

        if update_child {
            self.recompute_children(expanded_node, context);
        }
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Slot").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
