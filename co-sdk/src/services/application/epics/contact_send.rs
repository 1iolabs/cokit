// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	library::{contact::create_contact_message, settings_timeout::settings_timeout},
	services::application::action::ContactAction,
	Action, ActionError, CoContext, CO_ID_LOCAL,
};
use co_actor::Actions;
use co_identity::{IdentityResolver, PrivateIdentityResolver};
use co_network::identities_networks;
use co_primitives::CoId;
use futures::{pin_mut, FutureExt, Stream, StreamExt, TryStreamExt};

/// Send a contact request to a DID.
///
/// In: [`Action::Contact`]
/// Out: [`Action::ContactSent`]
pub fn contact_send(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::Contact(contact) => {
			let contact = contact.clone();
			let context = context.clone();
			Some(
				async move {
					let result = send_contact(&context, &contact).await.map_err(ActionError::from);
					Ok(Action::ContactSent(contact, result))
				}
				.into_stream(),
			)
		},
		_ => None,
	}
}

async fn send_contact(context: &CoContext, contact: &ContactAction) -> anyhow::Result<()> {
	let network = context.network().await.ok_or_else(|| anyhow::anyhow!("No network"))?;

	// resolve identities
	let from_identity = context
		.private_identity_resolver()
		.await?
		.resolve_private(&contact.from)
		.await?;
	let identity_resolver = context.identity_resolver().await?;
	let to_identity = identity_resolver.resolve(&contact.to).await?;

	// create message
	let (_header, message) = create_contact_message(
		context.date(),
		network.local_peer_id(),
		&from_identity,
		&to_identity,
		contact.sub.clone(),
		contact.fields.clone(),
	)?;

	// resolve networks for the recipient
	let networks = if contact.networks.is_empty() {
		let resolved: std::collections::BTreeSet<_> =
			identities_networks(Some(&identity_resolver), [contact.to.clone()])
				.try_collect()
				.await?;
		anyhow::ensure!(!resolved.is_empty(), "No networks found for recipient DID");
		resolved
	} else {
		contact.networks.clone()
	};

	// connect to the recipient's DID
	let connections = context
		.network_connections()
		.await
		.ok_or_else(|| anyhow::anyhow!("No network connections"))?;
	let peers_stream = co_network::connections::ConnectionMessage::did_use(
		connections,
		contact.from.clone(),
		contact.to.clone(),
		networks,
	);

	// get timeout
	let timeout = settings_timeout(context, &CoId::from(CO_ID_LOCAL), Some("didcomm-send")).await;

	// try to send to the first connectable peer
	let mut last_error: Option<anyhow::Error> = None;
	pin_mut!(peers_stream);
	while let Some(peers) = peers_stream.next().await {
		match peers {
			Ok(peers) => {
				for peer in peers.added {
					match network.didcomm_send([peer], message.clone(), timeout).await {
						Ok(_) => return Ok(()),
						Err(err) => {
							last_error = Some(err);
						},
					}
				}
			},
			Err(err) => {
				last_error = Some(err.into());
			},
		}
	}

	Err(last_error.unwrap_or_else(|| anyhow::anyhow!("No peers available for contact")))
}
