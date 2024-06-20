use crate::{discovery, heads};

/// Trait which can be implemented on NetworkBehaviours which provide gossipsub.
pub trait HeadsLayerBehaviourProvider {
	type Event;

	fn heads(&self) -> &heads::HeadsState;
	fn heads_mut(&mut self) -> &mut heads::HeadsState;

	/// Extract heads event from event.
	fn heads_event(event: &Self::Event) -> Option<&heads::Event>;
	fn into_heads_event(event: Self::Event) -> Result<heads::Event, Self::Event>;

	fn handle_event<F: Fn(&heads::Event) -> bool>(
		event: Self::Event,
		predicate: F,
	) -> Result<heads::Event, Self::Event> {
		match Self::heads_event(&event) {
			Some(behaviour_event) if predicate(behaviour_event) => Self::into_heads_event(event),
			_ => Err(event),
		}
	}
}

pub trait DiscoveryLayerBehaviourProvider<R> {
	type Event;

	fn discovery(&self) -> &discovery::DiscoveryState<R>;
	fn discovery_mut(&mut self) -> &mut discovery::DiscoveryState<R>;

	/// Extract discovery event from event.
	fn discovery_event(event: &Self::Event) -> Option<&discovery::Event>;
	fn into_discovery_event(event: Self::Event) -> Result<discovery::Event, Self::Event>;

	fn handle_event<F: Fn(&discovery::Event) -> bool>(
		event: Self::Event,
		predicate: F,
	) -> Result<discovery::Event, Self::Event> {
		match Self::discovery_event(&event) {
			Some(behaviour_event) if predicate(behaviour_event) => Self::into_discovery_event(event),
			_ => Err(event),
		}
	}
}
